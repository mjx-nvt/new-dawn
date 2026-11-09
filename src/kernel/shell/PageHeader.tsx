import type { ReactNode } from "react";

/** The one page grammar (D-090): eyebrow (the module or parent — never the workspace, the badge
 *  shows it; none when it would only repeat the title), the title, one meta line, and the
 *  actions at the right (one primary; the rest in ⋯). */
export default function PageHeader({ eyebrow, title, meta, actions }: { eyebrow?: string; title: ReactNode; meta?: ReactNode; actions?: ReactNode }) {
  return (
    <div className="pageHeader">
      <div className="pageHeaderText">
        {eyebrow && <p className="eyebrow">{eyebrow}</p>}
        <h1 className="pageTitle">{title}</h1>
        {meta && <p className="pageMeta">{meta}</p>}
      </div>
      {actions && <div className="pageActions">{actions}</div>}
    </div>
  );
}
