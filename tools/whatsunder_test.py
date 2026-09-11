import subprocess
import sys
from pathlib import Path

from whatsunder import labels_under, nodes

HERE = Path(__file__).parent


def node(label, bounds, quote='"'):
    return f"<node text={quote}{label}{quote} class=\"android.widget.Button\" bounds=\"{bounds}\" />"


def screen(*items):
    return "<hierarchy>" + "".join(items) + "</hierarchy>"


def run(xml, x, y):
    return subprocess.run(
        [sys.executable, str(HERE / "whatsunder.py"), str(x), str(y)],
        input=xml, capture_output=True, text=True,
    )


def main():
    fly = screen(node("Arm", "[100,200][300,400]"), node("Fly", "[0,900][200,1000]"))
    assert labels_under(fly, 200, 300) == ["Arm"]
    assert labels_under(fly, 50, 950) == []

    single = screen(node("Land", "[0,0][100,100]", quote="'"))
    assert labels_under(single, 50, 50) == ["Land"], "a single-quoted attribute must still be seen"

    planning = screen(
        node("Upload", "[0,0][100,100]"),
        node("Download", "[100,0][200,100]"),
        node("Land", "[0,200][100,300]"),
    )
    assert labels_under(planning, 50, 250) == [], "Land on the plan screen adds an item"

    sheet = screen(
        node("Plan items", "[0,0][400,60]"),
        node("Takeoff", "[0,200][400,300]"),
    )
    assert labels_under(sheet, 200, 250) == [], (
        "the plan item sheet covers Upload and Download, so it names itself instead"
    )

    assert nodes(fly), "the node pattern must match a real dump"

    done = run(fly, 200, 300)
    assert done.returncode == 0 and done.stdout.strip() == "Arm", done

    broken = run("not xml at all", "x", 300)
    assert broken.returncode != 0, "a guard that cannot answer must exit non-zero, not allow the tap"

    truncated = fly[: fly.index("<node text=\"Fly\"")]
    assert "</hierarchy>" not in truncated
    assert labels_under(truncated, 200, 300) == ["Arm"], (
        "a truncated dump still parses, which is why ui.sh checks for the closing tag "
        "rather than trusting a non-empty read"
    )

    print("whatsunder: ok")


if __name__ == "__main__":
    main()
