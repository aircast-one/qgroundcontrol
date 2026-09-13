import sys

from rig import (
    adding_after,
    close_list,
    coordinates,
    find,
    items,
    long_press,
    new_plan,
    open_list,
    sh,
    tap_label,
    TOOLS,
)

PLACES = [(400, 800), (650, 950), (500, 1100)]
NEW_PLACE = (900, 640)


def main():
    print("where a long-press puts the item. The list prints 'waypoint' for every row,")
    print("so an insert and an append look identical there - this compares coordinates.")

    new_plan()
    tap_label("Takeoff")
    for x, y in PLACES:
        long_press(x, y)

    if len(items()) != 2 + len(PLACES):
        raise SystemExit(f"PRECONDITION: wanted {2 + len(PLACES)} items, got {len(items())}")

    open_list()
    row = find("text=Waypoint")
    if not row:
        raise SystemExit("PRECONDITION: no Waypoint row in the plan list")
    sh(f"{TOOLS}/ui.sh tap {row}")
    close_list()

    selected = adding_after()
    if selected is None:
        raise SystemExit("PRECONDITION: nothing is selected, so there is no position to insert after")

    before = coordinates()
    if selected >= len(before) - 1:
        raise SystemExit(f"PRECONDITION: #{selected} is the last item, where insert and append agree")

    long_press(*NEW_PLACE)
    after = coordinates()

    if len(after) != len(before) + 1:
        raise SystemExit(f"PRECONDITION: the plan went from {len(before)} to {len(after)} items")

    known = {(lat, lon) for _, lat, lon in before}
    fresh = [seq for seq, lat, lon in after if (lat, lon) not in known]
    if len(fresh) != 1:
        raise SystemExit(f"PRECONDITION: expected one new coordinate, found {len(fresh)}")

    landed = fresh[0]
    want = selected + 1
    print(f"  selected #{selected}, so the new item belongs at #{want}")
    if landed == want:
        print(f"  ok   it landed at #{landed}")
        print("\nthe long-press inserts where the UI says it will")
        return
    print(f"  FAIL it landed at #{landed} of {len(after) - 1}")
    if landed == len(after) - 1:
        print('       that is the end of the plan, so the index never reached the insert')
    sys.exit(1)


main()
