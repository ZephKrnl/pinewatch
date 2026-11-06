"""Print the IL bodies of methods from an ikdasm dump.

Usage: python -I il_method.py <dump.il> <regex on "Class::Method"> [--calls]

With --calls, print only the call/newobj/ldstr/ldsfld lines of each body.
"""
import re
import sys

path, pattern = sys.argv[1], re.compile(sys.argv[2])
calls_only = "--calls" in sys.argv
lines = open(path, encoding="utf-8", errors="replace").read().splitlines()
end_re = re.compile(r"^\s*\} // end of method (\S+)$")
starts = []
for i, line in enumerate(lines):
    if line.lstrip().startswith(".method "):
        starts.append(i)
    m = end_re.match(line)
    if m and pattern.search(m.group(1)):
        start = starts[-1] if starts else i
        print(f"===== {m.group(1)} (lines {start + 1}-{i + 1})")
        for body_line in lines[start : i + 1]:
            if not calls_only or re.search(r"\b(call|callvirt|newobj|ldstr|ldsfld|stsfld)\b", body_line):
                print(body_line)
