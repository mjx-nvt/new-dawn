#!/usr/bin/env python3
"""One-off: turn the VIFC minutes template (a hand-filled form) into a New Dawn template.

Edits only text inside word/document.xml (formatting untouched): bracket hints that New Dawn fills
become {{fields}} / {{#blocks}}, worked examples ("e.g.") and the two "delete before issue"
instruction boxes are removed, one example row/paragraph is kept as the pattern to clone. The
minute-taker is the app's owner ({{minute_taker}}, {{minute_taker_name}}); hints the app can't know
(venue, chair, classification, register, circulation) stay as [ ] for the user to complete in Word
(D-023). Usage: vifc_template_placeholders.py ORIGINAL.docx OUT.docx
"""
import sys
import zipfile
from xml.dom import minidom

src, out = sys.argv[1], sys.argv[2]
with zipfile.ZipFile(src) as z:
    doc = minidom.parseString(z.read("word/document.xml"))


def texts(el):
    return [t for t in el.getElementsByTagName("w:t")]


def text(el):
    return "".join(t.firstChild.data for t in texts(el) if t.firstChild)


def set_text(el, value):
    """Put `value` in the first w:t of `el` (keeps its run formatting), empty the others."""
    ts = texts(el)
    assert ts, f"no text in element: {text(el)!r}"
    for i, t in enumerate(ts):
        while t.firstChild:
            t.removeChild(t.firstChild)
        t.setAttribute("xml:space", "preserve")
        if i == 0:
            t.appendChild(doc.createTextNode(value))


def set_runs(el, values):
    """Run-by-run text (keeps each run's own formatting, e.g. a bold prefix + normal text)."""
    runs = [r for r in el.getElementsByTagName("w:r") if r.getElementsByTagName("w:t")]
    assert len(runs) == len(values), f"{len(runs)} runs, {len(values)} values in {text(el)!r}"
    for run, value in zip(runs, values):
        set_text(run, value)


def replace(el, old, new):
    for t in texts(el):
        if t.firstChild and old in t.firstChild.data:
            t.firstChild.data = t.firstChild.data.replace(old, new)
            return
    raise AssertionError(f"{old!r} not found in {text(el)!r}")


def remove(el):
    el.parentNode.removeChild(el)


body = doc.getElementsByTagName("w:body")[0]
children = lambda node, tag: [c for c in node.childNodes if c.nodeType == c.ELEMENT_NODE and c.tagName == tag]
paras = lambda: children(body, "w:p")
tables = lambda: children(body, "w:tbl")


def para(starts):
    hits = [p for p in paras() if text(p).startswith(starts)]
    assert len(hits) == 1, f"{starts!r}: {len(hits)} paragraphs"
    return hits[0]


def table(contains):
    hits = [t for t in tables() if contains in text(t)]
    assert len(hits) == 1, f"{contains!r}: {len(hits)} tables"
    return hits[0]


def rows(tbl):
    return children(tbl, "w:tr")


def row(tbl, starts):
    hits = [r for r in rows(tbl) if text(r).startswith(starts)]
    assert len(hits) == 1, f"row {starts!r}: {len(hits)}"
    return hits[0]


def cells(tr):
    return children(tr, "w:tc")


# Instruction boxes ("Delete this box and the box below before issue").
remove(table("How to use this template"))
remove(table("Where minutes go wrong"))

# Guidance lines under each section heading, and worked examples.
for starts in [
    "Two or three sentences",
    "Group by organisation.",
    "One sub-heading per agenda item",
    "Only what was actually decided",
    "One owner — a named person",
    "Raised but not concluded",
    "e.g. The Hub met",
]:
    remove(para(starts))

# Title line.
set_text(para("[Meeting name]"), "{{title}}")

# Details table.
details = table("Date and time")
set_text(cells(row(details, "Date and time"))[1], "{{datetime}}")
set_text(cells(row(details, "Meeting type"))[1], "{{meeting_type}}")
agenda_cell = cells(row(details, "Agenda"))[1]
agenda_paras = children(agenda_cell, "w:p")
set_runs(agenda_paras[0], ["{{#agenda}}Session {{n}}: ", "{{topic}}{{/agenda}}"])
for p in agenda_paras[1:]:
    remove(p)
set_text(cells(row(details, "Reference no."))[1], "{{reference}}")
set_text(cells(row(details, "Minute-taker"))[1], "{{minute_taker}}")  # "Name — Title"

# Section 1.
set_text(para("[Summary of context"), "{{summary}}")
replace(table("Next focus"), "[One sentence on what must happen first. Details in Section 5.]", "{{next_focus}}")

# Section 2: attendees grouped by organisation.
attendees = table("Attendance")
group, member = row(attendees, "[Organisation 1"), rows(attendees)[2]
set_text(group, "{{#attendee_groups}}{{organisation}}")
c = cells(member)
set_text(c[0], "{{#members}}{{name}}")
set_text(c[1], "{{job_title}}")  # not {{title}}: that is the meeting title in the outer scope
set_text(c[2], "{{attendance}}{{/members}}{{/attendee_groups}}")
for r in rows(attendees)[3:]:
    remove(r)

# Section 3: one sub-heading per agenda item, one bullet per point.
heading = para("3.1  [Agenda item]")
set_text(heading, "{{#discussion}}3.{{n}}  {{topic}}")  # numbering is the template's; data gives n and nn (01)
bullet = heading.nextSibling
while bullet.nodeType != bullet.ELEMENT_NODE:
    bullet = bullet.nextSibling
assert text(bullet).startswith("•  [Point recorded"), text(bullet)
set_runs(bullet, ["•  ", "{{#points}}{{text}}{{/points}}{{/discussion}}"])
node = bullet.nextSibling
while node is not None and not text(node).startswith("SECTION 4"):
    nxt = node.nextSibling
    if node.nodeType == node.ELEMENT_NODE:
        remove(node)
    node = nxt

# Section 4: decisions.
decisions = table("Decided by")
first = row(decisions, "QĐ-01")
c = cells(first)
set_text(c[0], "{{#decisions}}QĐ-{{nn}}")
set_text(c[1], "{{text}}")
set_text(c[2], "{{decided_by}}")
set_text(c[3], "{{condition}}{{/decisions}}")
for r in rows(decisions):
    if text(r).startswith(("QĐ-02", "e.g.")):
        remove(r)

# Section 5: actions (no session grouping: the summary doesn't group actions).
actions = table("Owner")
for r in rows(actions):
    if text(r).startswith(("[Group", "CV-02", "CV-03", "CV-04", "e.g.")):
        remove(r)
c = cells(row(actions, "CV-01"))
set_text(c[0], "{{#actions}}CV-{{nn}}")
set_text(c[1], "{{item}}")
set_text(c[2], "{{owner}}")
set_text(c[3], "{{due}}")
set_text(c[4], "{{status}}{{/actions}}")

# Section 6: open issues.
set_runs(para("1.  [Open issue]"), ["{{#open_issues}}{{n}}.  ", "{{issue}} — ", "who can answer: ", "{{who}} — ", "by: ", "{{by}}{{/open_issues}}"])
remove(para("2.  [Open issue]"))
remove(para("3.  e.g."))

# Signatures: the minute-taker's name (left cell); the chair's stays a hint.
replace(cells(rows(table("Thư ký cuộc họp"))[0])[0], "[Full name]", "{{minute_taker_name}}")

xml = doc.toxml(encoding="UTF-8")
with zipfile.ZipFile(src) as zin, zipfile.ZipFile(out, "w", zipfile.ZIP_DEFLATED) as zout:
    for item in zin.infolist():
        data = xml if item.filename == "word/document.xml" else zin.read(item.filename)
        zout.writestr(item, data)
print("wrote", out)
