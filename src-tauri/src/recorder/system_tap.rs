//! The call's audio without a bot: a Core Audio global mono process tap (macOS 14.2+) inside a
//! private aggregate device that starts with the tap. The IO proc hands ~100 ms chunks to a
//! bounded channel with `try_send`, exactly like the mic.

use std::mem::ManuallyDrop;
use std::ptr::NonNull;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::sync::mpsc::SyncSender;

use anyhow::{Result, anyhow, bail};
use cidre::{arc, cat, cf, core_audio as ca, ns, os};

use super::mixer::Shared;

pub struct SystemTap {
    /// Dropped first in `Drop` (stops the IO proc), then the context it points to is freed.
    started: ManuallyDrop<ca::hardware::StartedDevice<ca::AggregateDevice>>,
    ctx: NonNull<TapContext>,
    _tap: ca::TapGuard,
    _agg_desc: arc::R<cf::DictionaryOf<cf::String, cf::Type>>,
    pub sample_rate: u32,
}

impl Drop for SystemTap {
    fn drop(&mut self) {
        // SAFETY: `started` is dropped exactly once, here; stopping the device guarantees the IO
        // proc no longer runs, so the context from `Box::into_raw` can be freed afterwards.
        unsafe {
            ManuallyDrop::drop(&mut self.started);
            drop(Box::from_raw(self.ctx.as_ptr()));
        }
    }
}

struct TapContext {
    tx: SyncSender<Vec<f32>>,
    shared: Arc<Shared>,
    pending: Vec<f32>,
    chunk: usize,
}

pub fn start(tx: SyncSender<Vec<f32>>, shared: Arc<Shared>) -> Result<SystemTap> {
    let output =
        ca::System::default_output_device().map_err(|e| anyhow!("no output device: {e:?}"))?;
    let output_uid = output
        .uid()
        .map_err(|e| anyhow!("output device uid: {e:?}"))?;

    let tap_desc = ca::TapDesc::with_mono_global_tap_excluding_processes(&ns::Array::new());
    let tap = tap_desc
        .create_process_tap()
        .map_err(|e| anyhow!("creating the process tap: {e:?}"))?;
    let asbd = tap
        .asbd()
        .map_err(|e| anyhow!("reading the tap format: {e:?}"))?;
    if asbd.bits_per_channel != 32 || !asbd.format_flags.contains(cat::AudioFormatFlags::IS_FLOAT) {
        bail!(
            "unexpected tap format: {} bits, flags {:?}",
            asbd.bits_per_channel,
            asbd.format_flags
        );
    }
    let tap_uid = tap.uid().map_err(|e| anyhow!("tap uid: {e:?}"))?;

    let sub_tap =
        cf::DictionaryOf::with_keys_values(&[ca::sub_device_keys::uid()], &[tap_uid.as_type_ref()]);
    let agg_desc = cf::DictionaryOf::with_keys_values(
        &[
            ca::aggregate_device_keys::is_private(),
            ca::aggregate_device_keys::is_stacked(),
            ca::aggregate_device_keys::tap_auto_start(),
            ca::aggregate_device_keys::name(),
            ca::aggregate_device_keys::main_sub_device(),
            ca::aggregate_device_keys::uid(),
            ca::aggregate_device_keys::tap_list(),
        ],
        &[
            cf::Boolean::value_true().as_type_ref(),
            cf::Boolean::value_false(),
            cf::Boolean::value_true(),
            cf::str!(c"New Dawn system audio").as_type_ref(),
            &output_uid,
            &cf::Uuid::new().to_cf_string(),
            &cf::ArrayOf::from_slice(&[sub_tap.as_ref()]),
        ],
    );
    let agg = ca::AggregateDevice::with_desc(&agg_desc)
        .map_err(|e| anyhow!("creating the aggregate device: {e:?}"))?;
    let sample_rate = agg.nominal_sample_rate().unwrap_or(asbd.sample_rate) as u32;

    let chunk = (sample_rate / 10).max(1) as usize;
    let ctx = Box::into_raw(Box::new(TapContext {
        tx,
        shared,
        pending: Vec::with_capacity(chunk * 2),
        chunk,
    }));
    // SAFETY: `ctx` comes from `Box::into_raw` and stays valid until `SystemTap::drop` (or the
    // error paths below) frees it, always after the device has stopped calling `io_proc`.
    let ctx = unsafe { NonNull::new_unchecked(ctx) };
    let proc_id = match agg.create_io_proc_id(io_proc, Some(unsafe { &mut *ctx.as_ptr() })) {
        Ok(id) => id,
        Err(e) => {
            drop(unsafe { Box::from_raw(ctx.as_ptr()) });
            bail!("registering the audio callback: {e:?}");
        }
    };
    let started = match ca::device_start(agg, Some(proc_id)) {
        Ok(started) => started,
        Err(e) => {
            drop(unsafe { Box::from_raw(ctx.as_ptr()) });
            bail!("starting system audio capture: {e:?}");
        }
    };
    tracing::info!(
        "system audio tap started at {sample_rate} Hz ({} ch)",
        asbd.channels_per_frame
    );

    Ok(SystemTap {
        started: ManuallyDrop::new(started),
        ctx,
        _tap: tap,
        _agg_desc: agg_desc,
        sample_rate,
    })
}

/// Core Audio IO proc (real-time thread): copy the tap's float samples, never block.
extern "C" fn io_proc(
    _device: ca::Device,
    _now: &cat::AudioTimeStamp,
    input: &cat::AudioBufList<1>,
    _input_time: &cat::AudioTimeStamp,
    _output: &mut cat::AudioBufList<1>,
    _output_time: &cat::AudioTimeStamp,
    ctx: Option<&mut TapContext>,
) -> os::Status {
    let Some(ctx) = ctx else {
        return os::Status::NO_ERR;
    };
    if input.number_buffers == 0 {
        return os::Status::NO_ERR;
    }
    let buf = &input.buffers[0];
    let channels = buf.number_channels.max(1) as usize;
    let floats = buf.data_bytes_size as usize / std::mem::size_of::<f32>();
    if floats > 0 && !buf.data.is_null() {
        // SAFETY: Core Audio gives a valid buffer of `data_bytes_size` bytes of f32 samples for the
        // duration of this call (the tap format is 32-bit float PCM).
        let data = unsafe { std::slice::from_raw_parts(buf.data as *const f32, floats) };
        ctx.pending.extend(
            data.chunks(channels)
                .map(|f| f.iter().sum::<f32>() / channels as f32),
        );
        // ponytail: one allocation per ~100 ms chunk on the real-time thread (like Meetily); move
        // to a preallocated ring buffer if dropouts ever show up.
        if ctx.pending.len() >= ctx.chunk {
            let full = std::mem::replace(&mut ctx.pending, Vec::with_capacity(ctx.chunk * 2));
            if ctx.tx.try_send(full).is_err() {
                ctx.shared.dropped.fetch_add(1, Ordering::Relaxed);
            }
        }
    }
    os::Status::NO_ERR
}
