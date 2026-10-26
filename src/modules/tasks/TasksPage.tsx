import { Link } from "@tanstack/react-router";
import { useEffect, useRef, useState, type KeyboardEvent as ReactKeyboardEvent, type ReactNode, type RefObject } from "react";
import { useProfile } from "../../kernel/bridge/profile";
import { useAddTask, useDeleteTask, useTasks, useUpdateTask, type TaskPatch, type TaskView } from "../../kernel/bridge/tasks";
import { useCurrentWorkspace } from "../../kernel/bridge/workspaces";
import DangerButton from "../../kernel/shell/DangerButton";
import { SparkleIcon } from "../../kernel/shell/icons";
import MenuButton from "../../kernel/shell/MenuButton";
import { dueLabel, isPast, quickDue, shortDate } from "./due";
import styles from "./TasksPage.module.css";

const NO_NAME = "Add your name in Settings → General so New Dawn can find your tasks in your meetings.";

/** An IME is still writing. WebKit ends the composition before the keydown of the Enter that
 *  confirms it (keyCode 229 then), so `isComposing` alone would save half-typed Vietnamese. */
const composing = (e: ReactKeyboardEvent) => e.nativeEvent.isComposing || e.keyCode === 229;

/** As a ref on the control an inline editor stands in for (or on its wrapper): once the editor
 *  closes from the keyboard or a calendar pick (`back` set), focus returns to that control
 *  instead of dropping to the page. */
const focusBack = (back: RefObject<boolean>) => (el: HTMLElement | null) => {
  if (!el || !back.current) return;
  back.current = false;
  (el.querySelector("button") ?? el).focus();
};

/** Tasks (D-042): this workspace's to-dos, as simple as Reminders — type one in, tick it, date it,
 *  rename it, delete it. The action items that are yours arrive after each new summary; the
 *  others come with "Add to Tasks" in the summary. */
export default function TasksPage() {
  const { data: workspace } = useCurrentWorkspace();
  const slug = workspace?.slug ?? "";
  const tasks = useTasks(workspace?.slug);
  const profile = useProfile();
  const [showDone, setShowDone] = useState(false);
  const addField = useRef<HTMLInputElement>(null);

  // Reminders' keys while this page is open: ⌘N goes to the add field, ⇧⌘H shows or hides Completed.
  useEffect(() => {
    function onKeyDown(e: KeyboardEvent) {
      if (!e.metaKey || e.altKey || e.ctrlKey || e.isComposing || document.querySelector("dialog[open]")) return;
      const key = e.key.toLowerCase();
      if (key === "n" && !e.shiftKey) addField.current?.focus();
      else if (key === "h" && e.shiftKey) setShowDone((shown) => !shown);
      else return;
      e.preventDefault();
    }
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, []);

  const list = tasks.data;
  const noName = profile.data?.name.trim() === "";
  const empty = !!list && list.today.length + list.upcoming.length + list.no_date.length + list.completed.length === 0;

  return (
    <div className="page">
      <p className="eyebrow">{workspace?.name ?? "No workspace"}</p>
      <h1 className="pageTitle">Tasks</h1>
      <div className={styles.board}>
        <AddField slug={slug} inputRef={addField} />
        {noName && !empty && <p className="fieldHint">{NO_NAME}</p>}
        {tasks.error && (
          <p className="formError" role="alert">
            {tasks.error.message}
          </p>
        )}

        <Section title="Today" slug={slug} tasks={list?.today}>
          {empty ? (
            <>
              <p className="ghostText">{noName ? NO_NAME : "Nothing yet. After each summary the action items that are yours arrive here; add your own above."}</p>
              <div className={styles.ghosts}>
                {[0, 1, 2].map((i) => (
                  <div key={i} className={`${styles.ghostRow} ghostCard`} />
                ))}
              </div>
            </>
          ) : (
            <p className="ghostText">Nothing due today</p>
          )}
        </Section>
        <Section title="Upcoming" slug={slug} tasks={list?.upcoming}>
          <p className="ghostText">Nothing coming up</p>
        </Section>
        <Section title="No date" slug={slug} tasks={list?.no_date}>
          <p className="ghostText">Nothing without a date</p>
        </Section>

        <details className={styles.section} open={showDone} onToggle={(e) => setShowDone(e.currentTarget.open)}>
          <summary className={`sectionTitle ${styles.heading}`} title="Show or hide completed tasks (⇧⌘H)">
            Completed ({list?.completed.length ?? 0})
          </summary>
          {list?.completed.length ? <Rows slug={slug} tasks={list.completed} /> : <p className="ghostText">Nothing completed yet</p>}
        </details>
      </div>
    </div>
  );
}

/** "Add a task": Enter adds it (no date) and the field stays ready for the next one. */
function AddField({ slug, inputRef }: { slug: string; inputRef: RefObject<HTMLInputElement | null> }) {
  const [draft, setDraft] = useState("");
  const add = useAddTask(slug);
  return (
    <>
      <input
        ref={inputRef}
        className="input"
        placeholder="Add a task"
        aria-label="Add a task"
        title="Add a task (⌘N)"
        maxLength={500}
        value={draft}
        disabled={!slug}
        onChange={(e) => setDraft(e.target.value)}
        onKeyDown={(e) => {
          const title = draft.trim();
          if (e.key !== "Enter" || composing(e) || !title) return;
          setDraft(""); // at once, so the next one can be typed while this one saves
          add.mutate(title, { onError: () => setDraft((typed) => typed || title) });
        }}
      />
      {add.error && (
        <p className="formError" role="alert">
          {add.error.message}
        </p>
      )}
    </>
  );
}

/** A section keeps its title when it's empty; `children` stand in for the rows then. */
function Section({ title, slug, tasks, children }: { title: string; slug: string; tasks: TaskView[] | undefined; children: ReactNode }) {
  return (
    <section className={styles.section}>
      <h2 className={`sectionTitle ${styles.heading}`}>{title}</h2>
      {tasks?.length ? <Rows slug={slug} tasks={tasks} /> : children}
    </section>
  );
}

function Rows({ slug, tasks }: { slug: string; tasks: TaskView[] }) {
  return (
    <ul className={styles.list}>
      {tasks.map((task) => (
        <Row key={task.id} slug={slug} task={task} />
      ))}
    </ul>
  );
}

/** One task: the circle, the title (click to rename), where it came from, Delete, the priority
 *  and due chips. */
function Row({ slug, task }: { slug: string; task: TaskView }) {
  const update = useUpdateTask(slug);
  const remove = useDeleteTask(slug);
  // A tick shows at once; the task moves to Completed a moment later (useUpdateTask), so a
  // mistaken tick can be undone in place.
  const [ticked, setTicked] = useState<boolean | null>(null);
  const done = ticked ?? !!task.done_at;
  const set = (patch: TaskPatch) => update.mutate({ id: task.id, patch });
  const name = task.owner ? `${task.owner}: ${task.title}` : task.title; // names the row's controls
  const said = !task.due_on && !!task.due_text; // a due phrase that names no day
  const why = task.reason || "Found in the meeting's action items";
  const error = update.error ?? remove.error;

  return (
    <li className={done ? `${styles.row} ${styles.done}` : styles.row}>
      <input
        type="checkbox"
        className={styles.check}
        checked={done}
        aria-label={name}
        onChange={(e) => {
          const next = e.target.checked;
          setTicked(next);
          update.mutate({ id: task.id, patch: { done: next } }, { onError: () => setTicked(null) });
        }}
      />
      <div className={styles.body}>
        <div className={styles.line}>
          {task.owner && <span className={styles.owner}>{task.owner}:</span>}
          <TitleField title={task.title} onSave={(title) => set({ title })} />
        </div>
        {(task.meeting || said || task.origin === "auto") && (
          <p className={styles.meta}>
            {task.meeting && (
              <Link to="/meetings/$meetingId" params={{ meetingId: task.meeting.id }} search={{ tab: "summary" }} className={styles.meeting}>
                {task.meeting.title} · {shortDate(new Date(task.meeting.held_on))}
              </Link>
            )}
            {said && <span>said “{task.due_text}”</span>}
            {task.origin === "auto" && (
              <span className="aiMark" title={why}>
                <SparkleIcon size={12} />
                <span className="srOnly">{why}</span>
              </span>
            )}
          </p>
        )}
        {error && (
          <p className="formError" role="alert">
            {error.message}
          </p>
        )}
      </div>
      <div className={styles.reveal}>
        <DangerButton confirm="Delete task" label={`Delete “${name}”`} onConfirm={() => remove.mutate(task.id)}>
          Delete
        </DangerButton>
      </div>
      <PriorityChip name={name} priority={task.priority} onSet={(priority) => set({ priority })} />
      <DueChip name={name} due={task.due_on} late={!done && !!task.due_on && isPast(task.due_on)} onSet={(due_on) => set({ due_on })} />
    </li>
  );
}

/** Reminders' levels, highest first: the value, its name, its marks. */
const PRIORITIES = [
  [3, "High", "!!!"],
  [2, "Medium", "!!"],
  [1, "Low", "!"],
] as const;

/** The priority as Reminders shows it — "!", "!!", "!!!" in orange, never red (red is overdue) —
 *  or a quiet "!" on hover when there's none. Its menu sets it. */
function PriorityChip({ name, priority, onSet }: { name: string; priority: number; onSet: (priority: number) => void }) {
  const level = PRIORITIES.find(([value]) => value === priority);
  return (
    <div className={level ? undefined : styles.reveal}>
      <MenuButton
        className={level ? `btn btnGhost btnCompact ${styles.priority}` : "btn btnGhost btnCompact"}
        label={level ? `Priority ${level[1]}: change the priority of “${name}”` : `Add a priority to “${name}”`}
        items={[
          ...PRIORITIES.map(([value, label, marks]) => ({ label: `${label} ${marks}`, onSelect: () => onSet(value) })),
          { label: "None", onSelect: () => onSet(0), disabled: !level, separator: true },
        ]}
      >
        {level?.[2] ?? "!"}
      </MenuButton>
    </div>
  );
}

/** The title; a click edits it in place — Enter or clicking away saves, Esc cancels; after Enter
 *  or Esc focus is back on the title. */
function TitleField({ title, onSave }: { title: string; onSave: (title: string) => void }) {
  const [editing, setEditing] = useState(false);
  const back = useRef(false);
  if (!editing) {
    return (
      <button type="button" ref={focusBack(back)} className={styles.title} onClick={() => setEditing(true)}>
        {title}
      </button>
    );
  }
  return (
    <input
      className={`input ${styles.titleInput}`}
      autoFocus
      defaultValue={title}
      maxLength={500}
      aria-label="Task title"
      onFocus={(e) => e.currentTarget.select()}
      onKeyDown={(e) => {
        if (composing(e)) return;
        if (e.key === "Escape") e.currentTarget.value = title; // so leaving saves nothing
        if (e.key === "Enter" || e.key === "Escape") {
          back.current = true;
          e.currentTarget.blur();
        }
      }}
      onBlur={(e) => {
        const next = e.currentTarget.value.trim();
        if (next && next !== title) onSave(next);
        setEditing(false);
      }}
    />
  );
}

/** The due day ("Today", "Fri", "3 Nov"; red when overdue) or "Add date". Its menu dates it the
 *  way To Do does: Today, Tomorrow, Next week, a day picked on the calendar, or none. */
function DueChip({ name, due, late, onSet }: { name: string; due: string | null; late: boolean; onSet: (due: string | null) => void }) {
  const [picking, setPicking] = useState(false);
  const back = useRef(false);
  if (picking) {
    const onClose = (refocus: boolean) => {
      back.current = refocus;
      setPicking(false);
    };
    return <DayField due={due} onPick={onSet} onClose={onClose} />;
  }
  const label = due ? dueLabel(due) : "Add date";
  return (
    <div ref={focusBack(back)} className={due ? undefined : styles.reveal}>
      <MenuButton
        className={late ? `btn btnGhost btnCompact ${styles.late}` : "btn btnGhost btnCompact"}
        label={due ? `Due ${label}: change the date of “${name}”` : `Add date to “${name}”`}
        items={[
          { label: "Today", onSelect: () => onSet(quickDue("today")) },
          { label: "Tomorrow", onSelect: () => onSet(quickDue("tomorrow")) },
          { label: "Next week", onSelect: () => onSet(quickDue("next_week")) },
          { label: "Pick a date…", onSelect: () => setPicking(true) },
          { label: "Remove date", onSelect: () => onSet(null), disabled: !due, separator: true },
        ]}
      >
        {label}
      </MenuButton>
    </div>
  );
}

/** "Pick a date…": the native date field with its calendar open, empty on a task without a date
 *  so leaving it untouched saves nothing. A day chosen on the calendar saves at once; a typed one
 *  on Enter or on leaving the field; Esc cancels. A pick, Enter and Esc hand focus back to the
 *  chip (`onClose(true)`); clicking elsewhere leaves it where it went. */
function DayField({ due, onPick, onClose }: { due: string | null; onPick: (day: string) => void; onClose: (refocus: boolean) => void }) {
  const field = useRef<HTMLInputElement>(null);
  const closed = useRef(false);
  const close = (day: string, refocus: boolean) => {
    if (closed.current) return;
    closed.current = true;
    if (day && day !== due) onPick(day);
    onClose(refocus);
  };
  // Once, on open.
  useEffect(() => {
    const el = field.current;
    if (!el) return;
    try {
      el.showPicker();
    } catch {
      // Not allowed here: the field still takes a typed date, and a click opens its calendar.
    }
    // The native change event (React's onChange fires on every keystroke): a calendar pick.
    const onChange = () => close(el.value, true);
    el.addEventListener("change", onChange);
    return () => el.removeEventListener("change", onChange);
  }, []);
  return (
    <input
      ref={field}
      type="date"
      className={`input ${styles.dayField}`}
      autoFocus
      defaultValue={due ?? ""}
      aria-label="Due date"
      onKeyDown={(e) => {
        if (e.key === "Enter") close(e.currentTarget.value, true);
        else if (e.key === "Escape") close("", true); // saves nothing
      }}
      onBlur={(e) => close(e.currentTarget.value, false)}
    />
  );
}
