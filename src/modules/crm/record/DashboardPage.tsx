import { useState } from "react";
import { useDashboardLayout, useSaveDashboardLayout } from "../../../kernel/bridge/dashboards";
import { recordId } from "../../../kernel/bridge/records";
import LayoutEditor from "./LayoutEditor";
import styles from "./Record.module.css";
import { WidgetBody, WidgetCard, type WidgetContext } from "./widgets";

/** A dashboard's page (D-067): the tabs and widgets of its own layout in view mode, or the
 *  layout editor with the dashboard palette while editing. */
export default function DashboardBody({ ctx, editing, onEditDone }: { ctx: WidgetContext; editing: boolean; onEditDone: () => void }) {
  const { slug, record } = ctx;
  const id = recordId(record);
  const layout = useDashboardLayout(slug, id);
  const save = useSaveDashboardLayout(slug, id);
  const [tab, setTab] = useState<string | null>(null);
  if (!layout.data) {
    return (
      <div className={styles.grid}>
        <div className={`${styles.widgetGhost} ghostCard`}>{layout.error ? <span className="ghostText">Couldn't be read — {layout.error.message}</span> : <span className="ghostText">Reading…</span>}</div>
      </div>
    );
  }
  if (editing) return <LayoutEditor ctx={ctx} layout={layout.data} kind="dashboard" onSave={(input) => save.mutateAsync(input)} onDone={onEditDone} />;
  const tabs = layout.data.tabs;
  const current = tabs.find((t) => t.tab.id === tab) ?? tabs.find((t) => t.tab.is_default) ?? tabs[0];
  return (
    <>
      {tabs.length > 1 && (
        <div className={`segmented ${styles.tabs}`} role="tablist" aria-label="Dashboard">
          {tabs.map((t) => (
            <button key={t.tab.id} type="button" role="tab" aria-selected={t.tab.id === current?.tab.id} onClick={() => setTab(t.tab.id)}>
              {t.tab.title}
            </button>
          ))}
        </div>
      )}
      <div role="tabpanel" className={styles.grid}>
        {current && current.widgets.length ? (
          current.widgets.map((w) => (
            <WidgetCard key={w.id} widget={w}>
              <WidgetBody ctx={ctx} widget={w} />
            </WidgetCard>
          ))
        ) : (
          <div className={`${styles.widgetGhost} ghostCard`}>
            <span className="ghostText">An empty dashboard — ⋯ → Edit dashboard adds charts, a table, text or a frame</span>
          </div>
        )}
      </div>
    </>
  );
}
