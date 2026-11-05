"""Extract a config string from tests/tests/sanity.rs exactly as Rust's format!
would produce it. Handles: {{ -> {, }} -> }, {MARKET_ACTIONS}, {name}
placeholders given as kwargs, and positional {} filled by constraint_rules(l_w)
whose argument is read from the source."""
import re, sys
SRC = open("/home/jayabratabasu/firma/tests/tests/sanity.rs").read()
MARKET = re.search(r'const MARKET_ACTIONS: &str = r#"(.*?)"#;', SRC, re.S).group(1)
CR = re.search(r'fn constraint_rules\(l_w: u32\) -> String \{\s*format!\(\s*r#"(.*?)"#', SRC, re.S).group(1)
def fmt(tmpl, positional, named):
    out, i, pos = [], 0, iter(positional)
    while i < len(tmpl):
        if tmpl.startswith("{{", i): out.append("{"); i += 2
        elif tmpl.startswith("}}", i): out.append("}"); i += 2
        elif tmpl[i] == "{":
            j = tmpl.index("}", i); name = tmpl[i+1:j]
            out.append(next(pos) if name == "" else str(named[name])); i = j + 1
        else: out.append(tmpl[i]); i += 1
    return "".join(out)
def constraint_rules(l_w): return fmt(CR, [], {"l_w": l_w})
def cfg(fn_name, **named):
    body = re.search(r"fn " + fn_name + r"\([^)]*\) -> String \{\s*format!\(\s*r#\"(.*?)\"#,\s*constraint_rules\((\d+)\)", SRC, re.S)
    tmpl, lw = body.group(1), int(body.group(2))
    named = {"MARKET_ACTIONS": MARKET, **named}
    return fmt(tmpl, [constraint_rules(lw)], named)
if __name__ == "__main__":
    fn, out = sys.argv[1], sys.argv[2]
    kw = dict(a.split("=", 1) for a in sys.argv[3:])
    open(out, "w").write(cfg(fn, **kw))
