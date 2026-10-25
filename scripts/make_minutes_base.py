#!/usr/bin/env python3
"""Dev-time: the neutral minutes base the app brands for every workspace (kernel/export/rebrand.rs),
made from the VIFC starter (brands/vifc/templates/meeting-minutes/template.docx, itself made by
vifc_template_placeholders.py). Edits text in place, formatting untouched (every target is one run):
VIFC's own words and rows go, the header slogans and the footer line become {{slogan_en}},
{{#slogan_vi}}…{{/slogan_vi}} (an empty Vietnamese slogan drops its line) and {{workspace_name}},
the footer's decorative picture goes, and the header logo becomes a 1×1 transparent PNG (the app
puts the workspace's logo there, or its name). The minute-taker's {{minute_taker}} and
{{minute_taker_name}} come with the VIFC starter; the checks at the end make sure of all of it.
Usage: python3 -I scripts/make_minutes_base.py
"""
import re
import struct
import xml.etree.ElementTree as ET
import zipfile
import zlib
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SRC = ROOT / "brands/vifc/templates/meeting-minutes/template.docx"
OUT = ROOT / "src-tauri/templates/minutes-base.docx"

with zipfile.ZipFile(SRC) as z:
    infos = z.infolist()
    parts = {i.filename: z.read(i.filename) for i in infos}
FILLED = [n for n in parts if n == "word/document.xml" or re.fullmatch(r"word/(header|footer)\d*\.xml", n)]


def text(name):
    return parts[name].decode("utf-8")


def put(name, xml):
    parts[name] = xml.encode("utf-8")


def set_t(xml, old, new):
    """The text of the one <w:t> holding exactly `old`."""
    needle = ">%s</w:t>" % old
    assert xml.count(needle) == 1, (old, xml.count(needle))
    return xml.replace(needle, ">%s</w:t>" % new)


def cut(xml, pattern, keep=lambda m: True, count=1):
    """Removes the `count` matches of `pattern` that `keep` accepts."""
    hits = [m for m in re.findall(pattern, xml, flags=re.S) if keep(m)]
    assert len(hits) == count, (pattern, len(hits))
    for hit in hits:
        xml = xml.replace(hit, "", 1)
    return xml


def runs_text(xml):
    return "".join(re.findall(r"<w:t(?: [^>]*)?>([^<]*)</w:t>", xml))


def target(rels):
    """The Target of the one relationship in a header/footer's rels (its picture)."""
    (found,) = re.findall(r'Target="([^"]+)"', rels)
    return "word/" + found


def png_1x1():
    chunk = lambda kind, data: struct.pack(">I", len(data)) + kind + data + struct.pack(">I", zlib.crc32(kind + data))
    ihdr = struct.pack(">IIBBBBB", 1, 1, 8, 6, 0, 0, 0)  # 1×1, 8-bit RGBA
    return b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", ihdr) + chunk(b"IDAT", zlib.compress(b"\0" * 5)) + chunk(b"IEND", b"")


def placeholders():
    found = set()
    for name in FILLED:
        found |= {m.strip().lstrip("#/").strip() for m in re.findall(r"\{\{([^{}]+)\}\}", text(name))}
    return found


vifc_fields = placeholders()

# Body: a plain eyebrow; VIFC's relationship register and partner follow-up rows go.
doc = text("word/document.xml")
doc = set_t(doc, "INTERNAL · GROUP CIRCULATION", "INTERNAL")
doc = set_t(doc, "Nội bộ · Lưu hành trong Tập đoàn", "Nội bộ")
doc = cut(doc, r"<w:tr>(?:(?!</w:tr>).)*</w:tr>", lambda row: runs_text(row).startswith(("Relationship register", "Summary to partner")), count=3)
put("word/document.xml", doc)

# Header: slogans from the brand pack; the logo picture is the app's to fill.
header = text("word/header1.xml")
header = set_t(header, "The convergence of global capital and Vietnam fintech", "{{slogan_en}}")
header = set_t(header, "Nơi hội tụ dòng vốn toàn cầu và fintech Việt Nam", "{{#slogan_vi}}{{slogan_vi}}{{/slogan_vi}}")
put("word/header1.xml", header)
parts[target(text("word/_rels/header1.xml.rels"))] = png_1x1()

# Footer: the workspace's name where VIFC's address was; the decorative picture goes.
footer = text("word/footer1.xml")
footer = cut(footer, r"<w:r><w:drawing>.*?</w:drawing></w:r>")
footer = set_t(footer, "Level 30, Saigon Marina IFC, 02 Ton Duc Thang, Saigon Ward, HCMC", "{{workspace_name}}")
footer = cut(footer, r"<w:p>(?:(?!</w:p>).)*Lầu 30(?:(?!</w:p>).)*</w:p>")
put("word/footer1.xml", footer)
rels = text("word/_rels/footer1.xml.rels")
del parts[target(rels)]
put("word/_rels/footer1.xml.rels", cut(rels, r"<Relationship [^>]*/>"))

core = text("docProps/core.xml")
for tag, value in [("dc:creator", "New Dawn"), ("cp:lastModifiedBy", "New Dawn"), ("dc:description", "")]:
    core, n = re.subn(r"<%s>[^<]*</%s>" % (tag, tag), "<%s>%s</%s>" % (tag, value, tag), core)
    assert n == 1, tag
put("docProps/core.xml", core)

for name, data in parts.items():
    if name.endswith((".xml", ".rels")):
        ET.fromstring(data)  # raises when a part no longer parses
    for word in ["VIFC", "Saigon Marina", "Lầu 30", "Level 30", "TEMPLATE_"]:
        assert word.encode("utf-8") not in data, (name, word)
added = {"minute_taker", "minute_taker_name", "slogan_en", "slogan_vi", "workspace_name"}
assert placeholders() == vifc_fields | added, placeholders() ^ (vifc_fields | added)

OUT.parent.mkdir(parents=True, exist_ok=True)
with zipfile.ZipFile(OUT, "w", zipfile.ZIP_DEFLATED) as zout:
    for info in infos:
        if info.filename in parts:
            zout.writestr(info, parts[info.filename])
print("wrote", OUT.relative_to(ROOT))
