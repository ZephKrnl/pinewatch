"""Rebuild Pinewatch's asset type table from an ikdasm dump of Assembly-CSharp.dll and diff it with table.rs.

Usage: python -I asset_classes.py <Assembly-CSharp.il> <table.rs>
"""
import re
import sys

il_path, table_path = sys.argv[1], sys.argv[2]

CATEGORIES = ["None", "Item", "Effect", "Object", "Resource", "Vehicle", "Animal", "Mythic", "Skin", "Spawn", "Npc"]

classes = {}  # full name -> dict(abstract, base, category_il, friendly)
stack = []  # (full name, brace depth at open)
depth = 0
pending = None  # class header being read (between .class and {)
current_method = None
method_body = []

class_re = re.compile(r"^\s*\.class\s+(.*)$")
with open(il_path, encoding="utf-8", errors="replace") as f:
    lines = f.read().splitlines()

i = 0
while i < len(lines):
    line = lines[i]
    m = class_re.match(line)
    if m:
        header = m.group(1)
        j = i + 1
        while not lines[j].strip().startswith("{"):
            header += " " + lines[j].strip()
            j += 1
        tokens = header.split()
        ext = None
        if "extends" in tokens:
            k = tokens.index("extends")
            ext = tokens[k + 1]
            if ext == "class":
                ext = tokens[k + 2]
        name_tokens = tokens[: tokens.index("extends")] if "extends" in tokens else tokens
        if "implements" in name_tokens:
            name_tokens = name_tokens[: name_tokens.index("implements")]
        name = name_tokens[-1]
        nested = "nested" in tokens
        if nested and stack:
            name = stack[-1][0] + "/" + name
        if ext:
            ext = re.sub(r"^\[[^\]]+\]", "", ext)
            ext = re.sub(r"<.*$", "", ext)
        classes[name] = {"abstract": "abstract" in name_tokens, "base": ext, "category_il": None, "friendly": False}
        stack.append((name, depth))
        depth += 1  # the class's opening brace
        i = j + 1
        continue
    stripped = line.strip()
    if stripped.startswith("{") and not stripped.startswith("{ //"):
        depth += stripped.count("{") - stripped.count("}")
    elif stripped.startswith("}"):
        depth -= 1
        if stack and depth == stack[-1][1]:
            stack.pop()
    if stack and ("get_assetCategory() cil managed" in line):
        body = []
        j = i
        while "end of method" not in lines[j]:
            body.append(lines[j])
            j += 1
        ops = [b.split(":", 1)[1].strip() for b in body if re.match(r"\s*IL_[0-9a-f]{4}:", b)]
        classes[stack[-1][0]]["category_il"] = ops
    if stack and ("get_FriendlyName() cil managed" in line):
        classes[stack[-1][0]]["friendly"] = True
    i += 1


def derives_from(name, root):
    seen = set()
    while name and name not in seen:
        if name == root:
            return True
        seen.add(name)
        name = classes.get(name, {}).get("base")
    return False


def category(name):
    while name:
        ops = classes[name]["category_il"]
        if ops is not None:
            if len(ops) == 2 and ops[1] == "ret" and ops[0].startswith("ldc.i4"):
                v = ops[0].split(".")[-1] if " " not in ops[0] else ops[0].split()[-1]
                v = int(v, 0) if v.lstrip("-").isdigit() or v.startswith("0x") else v
                return CATEGORIES[int(v)]
            return "IL:" + "; ".join(ops)
        name = classes[name]["base"]
    return "?"


def friendly_owner(name):
    while name:
        if classes[name]["friendly"]:
            return name
        name = classes[name]["base"]
    return None


assets = sorted(n for n in classes if n != "SDG.Unturned.Asset" and derives_from(n, "SDG.Unturned.Asset"))
shipped = {}
for n in assets:
    ns, _, simple = n.rpartition(".")
    shipped[simple] = {
        "ns": ns,
        "abstract": classes[n]["abstract"],
        "category": category(n),
        "item": derives_from(n, "SDG.Unturned.ItemAsset"),
        "friendly": friendly_owner(n),
    }

print(f"shipped Assembly-CSharp: {len(assets)} classes deriving from SDG.Unturned.Asset")
print("Asset.get_assetCategory:", classes["SDG.Unturned.Asset"]["category_il"])
print("FriendlyName overrides:", sorted(n for n in assets + ["SDG.Unturned.Asset"] if classes[n]["friendly"]))

NS = {"UNTURNED": "SDG.Unturned", "FOLIAGE": "SDG.Framework.Foliage"}
table = {}
for m in re.finditer(r'info\((\w+), "(\w+)", (\w+), (true|false), (true|false), (\w+)\)', open(table_path, encoding="utf-8").read()):
    table[m.group(2)] = {
        "ns": NS[m.group(1)],
        "category": m.group(3),
        "abstract": m.group(4) == "true",
        "item": m.group(5) == "true",
        "friendly_rule": m.group(6),
    }
print(f"table.rs: {len(table)} classes")

missing = sorted(set(shipped) - set(table))
extra = sorted(set(table) - set(shipped))
print("in the DLL, not in table.rs:", missing)
print("in table.rs, not in the DLL:", extra)
for simple in missing:
    print("  ", simple, shipped[simple])
mismatch = 0
for simple in sorted(set(shipped) & set(table)):
    s, t = shipped[simple], table[simple]
    for key in ("ns", "abstract", "category", "item"):
        if s[key] != t[key]:
            mismatch += 1
            print(f"MISMATCH {simple}.{key}: dll={s[key]!r} table={t[key]!r}")
print(f"{mismatch} field mismatches among the shared classes")

# FriendlyName rule: which class's override applies.
RULE_OWNER = {
    "Base": "SDG.Unturned.Asset",
    "ItemName": "SDG.Unturned.ItemAsset",
    "LandscapeMaterial": "SDG.Unturned.LandscapeMaterialAsset",
    "Tag": "SDG.Unturned.TagAsset",
    "Vendor": "SDG.Unturned.VendorAsset",
    "ServerListCuration": "SDG.Unturned.ServerListCurationAsset",
}
for simple in sorted(set(shipped) & set(table)):
    rule = table[simple]["friendly_rule"]
    owner = shipped[simple]["friendly"]
    if rule == "Localized":
        ok = owner is not None and owner.rpartition(".")[2] in {"AnimalAsset", "ObjectAsset", "ResourceAsset", "RoadAsset", "VehicleAsset"}
    else:
        ok = owner == RULE_OWNER[rule]
    if not ok:
        print(f"FRIENDLY {simple}: table rule {rule}, DLL override in {owner}")
