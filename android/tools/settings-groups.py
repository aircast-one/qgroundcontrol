import pathlib
import re
import sys

ROOT = pathlib.Path(__file__).resolve().parents[2]
GROUPS = ROOT / "groundstation/src/settingsgroups.rs"
PAGES = ROOT / "groundstation/src/settings.rs"
HEAD_TREE = ROOT / "android/app/src/main/java"
HEAD = HEAD_TREE / "one/aircast/android/ui/SettingsScreen.kt"
SHOWN_WITHOUT_SECTIONS = ("shows_links", "shows_about", "shows_console", "shows_px4_logs", "shows_video_sources")


def core_groups():
    return re.findall(r'Group \{\s*name: "([^"]+)"', GROUPS.read_text())


def core_constants(text):
    return dict(re.findall(r'^(?:pub )?const (\w+): &str = "([^"]*)";', text, re.M))


def core_pages():
    text = PAGES.read_text()
    constants = core_constants(text)
    table = text[text.index("const PAGES:"):text.index("const HIDDEN:")]
    unlisted = re.search(r"const UNLISTED: &\[&str\] = &\[([^\]]*)\]", text).group(1)
    hidden = {constants.get(name.strip(), name.strip().strip('"')) for name in unlisted.split(",") if name.strip()}
    pages = []
    for body in re.split(r"\n    Page \{", table)[1:]:
        title = re.search(r'title: (?:"([^"]+)"|(\w+)),', body)
        name = title.group(1) or constants[title.group(2)]
        sections = re.search(r"sections: &\[(.*?)\],\n", body, re.S).group(1)
        groups = re.findall(r'\("[^"]*",\s*"(\w+)"\)', sections)
        shown = any(re.search(rf"{flag}: true", body) for flag in SHOWN_WITHOUT_SECTIONS)
        pages.append((name, groups, shown, name in hidden))
    return pages


def head_constants():
    return dict(
        found
        for path in HEAD_TREE.rglob("*.kt")
        for found in re.findall(r'^internal const val (\w+) = "([^"]*)"', path.read_text(), re.M)
    )


def head_block(name, opener):
    text = HEAD.read_text()
    block = text[text.index(f"internal val {name} = {opener}("):]
    return block[:block.index(")\n")] if opener == "setOf" else block[:block.index("\n)")]


def head_keys(name, constants):
    found = re.findall(r'^    (?:"([^"]+)"|(\w+)) to', head_block(name, "mapOf"), re.M)
    return {literal or constants.get(constant, constant) for literal, constant in found}


def head_members(name, constants):
    members = head_block(name, "setOf").split("(", 1)[1]
    return {constants.get(member.strip(), member.strip().strip('"')) for member in members.split(",") if member.strip()}


def stem(name):
    run = re.match(r"[A-Z]+", name).group()
    keep = len(run) - 1 if len(run) > 1 and len(run) < len(name) else len(run)
    return name[:keep].lower() + name[keep:]


def main():
    constants = head_constants()
    skipped_pages = head_keys("PAGES_WITHOUT_A_SCREEN", constants)
    looks = head_keys("PAGE_LOOKS", constants)
    drawn_by_head = head_members("SECTIONS_DRAWN_BY_THE_HEAD", constants)
    pages = core_pages()
    carried = {}
    for title, groups, _, _ in pages:
        for group in groups:
            carried.setdefault(group, []).append(title)

    unexplained, absent = [], []
    for name in core_groups():
        key = stem(name) + "Settings"
        where = carried.get(key) or carried.get(stem(name))
        if where is None:
            unexplained.append(f"{name} is served but no page in settings.rs carries it")
        elif all(title in skipped_pages for title in where):
            absent.append(name)

    offered = [title for title, groups, shown, unlisted in pages
               if not unlisted and title not in skipped_pages and (groups or shown)]
    unexplained += [f"page {title} has no entry in PAGE_LOOKS" for title in offered if title not in looks]

    stale = [f"section {name} in SECTIONS_DRAWN_BY_THE_HEAD" for name in drawn_by_head if name not in carried]
    stale += [f"page {title} in PAGES_WITHOUT_A_SCREEN" for title in skipped_pages
              if title not in {t for t, _, _, _ in pages}]

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
