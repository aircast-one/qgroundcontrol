import sys

from PIL import Image

path = sys.argv[1]
try:
    grey = Image.open(path).convert("L")
except Exception as problem:
    print("FAIL: %s is not an image (%s)" % (path, problem), file=sys.stderr)
    raise SystemExit(1)

low, high = grey.getextrema()
if high - low < 32:
    print("FAIL: %s spans %d-%d - screen asleep or blank?" % (path, low, high), file=sys.stderr)
    raise SystemExit(1)
print("ok %s (%d-%d)" % (path, low, high))
