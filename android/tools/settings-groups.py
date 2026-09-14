import pathlib
import re
import sys

ROOT = pathlib.Path(__file__).resolve().parents[2]
GROUPS = ROOT / "core-rs/src/settingsgroups.rs"
PAGES = ROOT / "core-rs/src/settings.rs"
HEAD = ROOT / "android/app/src/main/java/one/aircast/android/ui/SettingsScreen.kt"


def core_groups():
    return re.findall(r'Group \{ name: "([^"]+)"', GROUPS.read_text())


def core_pages():
    text = PAGES.read_text()
    table = text[text.index("const PAGES:"):text.index("const HIDDEN:")]
    return [
        (title, re.findall(r'\("[^"]*", "(\w+)"\)', sections))
        for title, sections in re.findall(r'Page \{ title: "([^"]+)", sections: &\[([^\]]*)\]', table)
    ]


def head_set(name):
    text = HEAD.read_text()
    block = text[text.index(f"internal val {name} = mapOf("):]
    return set(re.findall(r'^    "([^"]+)" to', block[:block.index("\n)")], re.M))


def head_notes():
    text = HEAD.read_text()
    block = text[text.index("internal val PAGE_NOTES = mapOf("):]
    return set(re.findall(r'^    "([^"]+)" to', block[:block.index("\n)")], re.M))


def stem(name):
    run = re.match(r"[A-Z]+", name).group()
    keep = len(run) - 1 if len(run) > 1 and len(run) < len(name) else len(run)
    return name[:keep].lower() + name[keep:]


# The head draws every page and every section view.settings serves, so what a group needs to be
# reachable is a page that carries it. What it needs to be DELIBERATELY absent is an entry in one
# of the head's two exclusion tables, each of which states its reason beside the name.
def main():
    skipped_pages, skipped_sections, notes = (
        head_set("PAGES_WITHOUT_A_SCREEN"), head_set("SECTIONS_WITHOUT_A_SCREEN"), head_notes())
    pages = core_pages()
    carried = {}
    for title, groups in pages:
        for group in groups:
            carried.setdefault(group, []).append(title)

    unexplained, absent = [], []
    for name in core_groups():
        key = stem(name) + "Settings"
        where = carried.get(key) or carried.get(stem(name))
        if where is None:
            unexplained.append(f"{name} is served but no page in settings.rs carries it")
        elif key in skipped_sections or all(title in skipped_pages for title in where):
            absent.append(name)

    offered = [title for title, groups in pages
               if title not in skipped_pages and any(g not in skipped_sections for g in groups)]
    unexplained += [f"page {title} has no line in PAGE_NOTES" for title in offered if title not in notes]

    stale = [f"section {name}" for name in skipped_sections if name not in carried]
    stale += [f"page {title}" for title in skipped_pages if title not in {t for t, _ in pages}]

    for name in absent:
        print(f"  {name:24} left out with a reason")
    for why in unexplained:
        print(f"  UNEXPLAINED {why}")
    for name in stale:
        print(f"  STALE {name} is not served any more - drop its entry")
    print(f"{len(core_groups())} groups served, {len(absent)} left out with a reason, "
          f"{len(unexplained)} unexplained")
    return 1 if unexplained or stale else 0


if __name__ == "__main__":
    sys.exit(main())
