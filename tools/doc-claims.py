import re
import sys

SECTION = re.compile(r"^#{2,3} (Status|Where the [\w ]+ stands|Known gaps|What is left|Remaining)\b", re.I)
HEADING = re.compile(r"^#{2,3} ")
CLAIM = re.compile(
    r"\b(is not|are not|is still|are still|cannot|can not|no core view|no head|undrawn|unread|"
    r"not wired|not drawn|never seen|unimplemented|missing|still QML|not yet|only \w+-only|"
    r"metres-only|deliberately not|there is no|there are no|has no|have no)\b",
    re.I,
)
STRUCK = re.compile(r"~~.+?~~", re.S)


def sections(lines):
    starts = [i for i, line in enumerate(lines) if SECTION.match(line)]
    for start in starts:
        body = []
        for line in lines[start + 1:]:
            if HEADING.match(line):
                break
            body.append(line)
        yield start + 1, lines[start], body


def claims(body):
    text = STRUCK.sub(" ", " ".join(body))
    sentences = re.split(r"(?<=[.;])\s+", text)
    return [s.strip() for s in sentences if CLAIM.search(s) and len(s.strip()) > 18]


def main():
    if len(sys.argv) < 2:
        raise SystemExit("usage: doc-claims.py <plan.md>")
    lines = open(sys.argv[1]).read().split("\n")
    total = 0
    print("Claims in the present tense, from status sections only. Each one asserts something")
    print("about the code as it is now, and nothing expires it. Check them, do not trust them.")
    print("Expect false positives: a bullet whose headline states a principle and whose body")
    print("then says it was fixed reads here as an open claim. Read around the line before acting.\n")
    for line_no, heading, body in sections(lines):
        found = claims(body)
        if not found:
            continue
        print(f"{sys.argv[1]}:{line_no}  {heading.strip()[:70]}")
        for claim in found:
            total += 1
            print(f"    - {claim[:190]}")
        print()
    print(f"{total} claims to check.")
    print("Struck-through lines are skipped, so mark a settled claim with ~~ rather than deleting it.")


main()
