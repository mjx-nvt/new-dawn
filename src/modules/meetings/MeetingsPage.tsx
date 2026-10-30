import { getRouteApi, Link, useNavigate } from "@tanstack/react-router";
import { useEffect, useState } from "react";
import { useMeetings, useMeetingTypes, useSearchByMeaning, useSearchMeetings, useTranscriptionProgress, type Meeting, type MeetingHit } from "../../kernel/bridge/meetings";
import { openSettings } from "../../kernel/bridge/settings";
import { openOllamaApp, useOllamaStatus } from "../../kernel/bridge/summary";
import { SparkleIcon } from "../../kernel/shell/icons";
import { useCurrentWorkspace } from "../../kernel/bridge/workspaces";
import { formatClock, formatWhen } from "./format";
import ImportAudioSheet from "./ImportAudioSheet";
import {
  SORT_LABEL,
  SORTS,
  STATUS_FILTER_LABEL,
  STATUS_FILTERS,
  WHEN_FILTER_LABEL,
  WHEN_FILTERS,
  applyFilters,
  groupMeetings,
  parseDateQuery,
  type Filters,
  type Sort,
  type StatusFilter,
  type WhenFilter,
} from "./listing";
import styles from "./MeetingsPage.module.css";
import { SummaryBar, TranscriptionBar } from "./AiProgress";
import { useSummaryProgress } from "../../kernel/bridge/summary";
import Snippet from "./Snippet";
import StatusChip from "./StatusChip";

const route = getRouteApi("/shell/meetings");

export type MeetingsSearch = { import?: true; q?: string; status?: StatusFilter; type?: string; when?: WhenFilter; sort?: Sort };

/** The current workspace's meetings grouped by date, with filters, sort and search (text and
 *  dates) — all in the URL, so Back/Forward and the toolbar search keep working. */
export default function MeetingsPage() {
  const { data: workspace } = useCurrentWorkspace();
  const search = route.useSearch();
  const { import: importOpen, q = "" } = search;
  const filters: Filters = { status: search.status ?? "all", type: search.type ?? "", when: search.when ?? "all", sort: search.sort ?? "newest" };
  const navigate = useNavigate();
  const types = useMeetingTypes(workspace?.slug);
  const meetings = useMeetings(workspace?.slug);
  const { range, rest } = parseDateQuery(q);
  const textSearch = useSearchMeetings(workspace?.slug, rest);
  const searching = rest.trim().length > 0;
  // By meaning, once typing pauses (each query is a call to the embedding model).
  const [meaningQuery, setMeaningQuery] = useState(rest.trim());
  useEffect(() => {
    const t = setTimeout(() => setMeaningQuery(rest.trim()), 400);
    return () => clearTimeout(t);
  }, [rest]);
  const ollama = useOllamaStatus();
  const embedReady = !!ollama.data?.running && !!ollama.data.embed_installed;
  const meaning = useSearchByMeaning(workspace?.slug, meaningQuery, searching && embedReady);

  const set = (next: Partial<MeetingsSearch>, replace = true) => {
    const merged: MeetingsSearch = { ...search, ...next };
    // Defaults stay out of the URL.
    const clean: MeetingsSearch = {};
    if (merged.import) clean.import = true;
    if (merged.q) clean.q = merged.q;
    if (merged.status && merged.status !== "all") clean.status = merged.status;
    if (merged.type) clean.type = merged.type;
    if (merged.when && merged.when !== "all") clean.when = merged.when;
    if (merged.sort && merged.sort !== "newest") clean.sort = merged.sort;
    void navigate({ to: "/meetings", search: clean, replace });
  };

  // Text search gives hits (meeting + snippet); otherwise every meeting. Filters and the searched
  // date then narrow either, so "Oct 2026 Wired Score" is that month's meetings mentioning it.
  const hits = searching ? (textSearch.data ?? []) : (meetings.data ?? []).map((meeting) => ({ meeting, snippet: null, at_ms: null }));
  const byId = new Map(hits.map((h) => [h.meeting.id, h]));
  const shown = applyFilters(
    hits.map((h) => h.meeting),
    filters,
    range,
  );
  const grouped = filters.sort === "newest" || filters.sort === "oldest";
  const groups = grouped ? groupMeetings(shown) : [{ label: SORT_LABEL[filters.sort], items: shown }];
  const loading = searching ? textSearch.isPending : meetings.isPending;
  const error = searching ? textSearch.error : meetings.error;
  const filtering = filters.status !== "all" || filters.type !== "" || filters.when !== "all" || !!range || searching;
  // By meaning: what the words didn't find, narrowed by the same filters.
  const meaningHits = meaning.data?.hits ?? [];
  const kept = new Set(applyFilters(meaningHits.map((h) => h.meeting), filters, range).map((m) => m.id));
  const related = meaningHits.filter((h) => kept.has(h.meeting.id) && !byId.has(h.meeting.id));
  const total = meetings.data?.length ?? 0;

  return (
    <div className="page">
      <div className={styles.header}>
        <div>
          <p className="eyebrow">{workspace?.name ?? "No workspace"}</p>
          <h1 className="pageTitle">Meetings</h1>
        </div>
        <button type="button" className="btn btnPrimary" disabled={!workspace} onClick={() => set({ import: true }, false)}>
          Import audio…
        </button>
      </div>

      <div className={styles.filters} role="group" aria-label="Filter and sort">
        <Select label="Status" value={filters.status} onChange={(v) => set({ status: v as StatusFilter })}>
          {STATUS_FILTERS.map((s) => (
            <option key={s} value={s}>
              {STATUS_FILTER_LABEL[s]}
            </option>
          ))}
        </Select>
        <Select label="Type" value={filters.type} onChange={(v) => set({ type: v })}>
          <option value="">All types</option>
          {types.data?.map((t) => (
            <option key={t.id} value={t.id}>
              {t.name}
            </option>
          ))}
          <option value="none">No type</option>
        </Select>
        <Select label="When" value={filters.when} onChange={(v) => set({ when: v as WhenFilter })}>
          {WHEN_FILTERS.map((w) => (
            <option key={w} value={w}>
              {WHEN_FILTER_LABEL[w]}
            </option>
          ))}
        </Select>
        <span className={styles.spacer} />
        <Select label="Sort" value={filters.sort} onChange={(v) => set({ sort: v as Sort })}>
          {SORTS.map((s) => (
            <option key={s} value={s}>
              {SORT_LABEL[s]}
            </option>
          ))}
        </Select>
      </div>

      <div className={styles.summary} aria-live="polite">
        <span className="fieldHint">
          {loading
            ? "Loading…"
            : filtering
              ? `${shown.length} of ${total} meeting${total === 1 ? "" : "s"}${range ? ` · ${range.label}` : ""}${searching ? ` · “${rest.trim()}”` : ""}`
              : `${total} meeting${total === 1 ? "" : "s"}`}
        </span>
        {filtering && (
          <button type="button" className={styles.clear} onClick={() => set({ q: undefined, status: "all", type: "", when: "all" })}>
            Show all
          </button>
        )}
      </div>
      {error && <p className={styles.error}>{error.message}</p>}

      {shown.length ? (
        groups.map((group) => (
          <section key={group.label} className={styles.group}>
            <h2 className={styles.groupTitle}>{group.label}</h2>
            <div className={styles.list}>
              {group.items.map((meeting) => {
                const hit = byId.get(meeting.id);
                return <MeetingRow key={meeting.id} meeting={meeting} snippet={hit?.snippet} atMs={hit?.at_ms} />;
              })}
            </div>
          </section>
        ))
      ) : (
        <section className={styles.group}>
          <p className="ghostText">
            {loading ? "Loading…" : related.length ? "No meeting uses these words" : filtering ? "No meetings match" : "Not yet captured — record a call or import audio"}
          </p>
          {!related.length && (
            <div className={styles.list}>
              {[0, 1, 2].map((i) => (
                <div key={i} className={`${styles.slot} ghostCard`} />
              ))}
            </div>
          )}
        </section>
      )}

      {searching && rest.trim().length >= 3 && ollama.data && (
        <ByMeaning
          hits={related}
          state={!ollama.data.running ? "no-ollama" : !ollama.data.embed_installed ? "no-model" : "ready"}
          indexed={meaning.data ? [meaning.data.indexed, meaning.data.meetings] : null}
          loading={meaning.isFetching}
          error={meaning.error?.message ?? null}
        />
      )}

      {importOpen && <ImportAudioSheet onClose={() => set({ import: undefined })} />}
    </div>
  );
}

function Select({ label, value, onChange, children }: { label: string; value: string; onChange: (v: string) => void; children: React.ReactNode }) {
  return (
    <label className={styles.filter}>
      <span className="srOnly">{label}</span>
      <select className="input" value={value} onChange={(e) => onChange(e.target.value)}>
        {children}
      </select>
    </label>
  );
}

interface RowProps {
  meeting: Meeting;
  snippet?: string | null;
  atMs?: number | null;
}

function MeetingRow({ meeting, snippet, atMs }: RowProps) {
  const progress = useTranscriptionProgress(meeting.id);
  const summaryProgress = useSummaryProgress(meeting.id);
  return (
    <Link
      to="/meetings/$meetingId"
      params={{ meetingId: meeting.id }}
      search={atMs != null ? { tab: "transcript", t: atMs } : { tab: snippet ? "summary" : "transcript" }}
      className={`card ${styles.row}`}
    >
      <div className={styles.main}>
        <p className={styles.title}>{meeting.title}</p>
        <p className={styles.meta}>
          {formatWhen(meeting.held_on)}
          {meeting.duration_ms != null && ` · ${formatClock(meeting.duration_ms)}`}
          {atMs != null && ` · match at ${formatClock(atMs)}`}
        </p>
        {snippet && <Snippet text={snippet} />}
        {progress && <TranscriptionBar progress={progress} title={meeting.title} />}
        {!progress && summaryProgress && <SummaryBar progress={summaryProgress} title={meeting.title} />}
      </div>
      <StatusChip status={meeting.status} />
    </Link>
  );
}

/** Search by meaning (D-053): the meetings close to the search that its words didn't find —
 *  found by the embedding model on this Mac, so they carry the AI marker. Says why when it can't
 *  look (Ollama not running, the model not downloaded) and how much is indexed so far. */
function ByMeaning({
  hits,
  state,
  indexed,
  loading,
  error,
}: {
  hits: MeetingHit[];
  state: "no-ollama" | "no-model" | "ready";
  indexed: [number, number] | null;
  loading: boolean;
  error: string | null;
}) {
  return (
    <section className={styles.group}>
      <h2 className={styles.groupTitle}>Related by meaning</h2>
      {state === "no-ollama" ? (
        <p className="fieldHint">
          Searching by meaning needs Ollama running on this Mac.{" "}
          <button type="button" className={styles.clear} onClick={() => void openOllamaApp()}>
            Open Ollama
          </button>
        </p>
      ) : state === "no-model" ? (
        <p className="fieldHint">
          Finds meetings by what was meant, in Vietnamese or English, even without the words.{" "}
          <button type="button" className={styles.clear} onClick={() => void openSettings("models")}>
            Download bge-m3 in Settings → Models
          </button>
        </p>
      ) : (
        <>
          <p className="fieldHint aiMark">
            <SparkleIcon /> Found by meaning, not by the words — bge-m3 on this Mac
            {indexed && indexed[0] < indexed[1] ? ` · ${indexed[0]} of ${indexed[1]} meetings indexed so far, the rest when your Mac is free` : ""}
          </p>
          {error && <p className="formError">{error}</p>}
          {hits.length ? (
            <div className={styles.list}>
              {hits.map((h) => (
                <MeetingRow key={h.meeting.id} meeting={h.meeting} snippet={h.snippet} atMs={h.at_ms} />
              ))}
            </div>
          ) : (
            <>
              <p className="ghostText">{loading ? "Looking…" : "Nothing else close in meaning"}</p>
              <div className={styles.list}>
                {[0, 1].map((i) => (
                  <div key={i} className={`${styles.slot} ghostCard`} />
                ))}
              </div>
            </>
          )}
        </>
      )}
    </section>
  );
}
