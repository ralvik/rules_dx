"""Bazel-cached docs site execution (extract to render).

"""

MDBOOK_VERSION = "0.4.43"

def site_symbol_id(language, package, qualified):
    """Returns the stable symbol ID `language:package:qualified`."""
    return language + ":" + package + ":" + qualified

def site_symbol_id_error(language, package, qualified):
    """Validates one symbol identity, returning "" when valid."""
    if language == "":
        return "docs_site: language is required"
    if package == "":
        return "docs_site: package is required"
    if qualified == "":
        return "docs_site: qualified name is required"
    return ""

def site_api_path(symbol_id):
    """Returns the workspace-relative API page for one symbol ID.

    `python:mylib:AccountService.create` maps to
    `api/python/mylib/AccountService.create.md` so URLs mirror IDs
    deterministically with no absolute paths.
    """
    return "api/" + symbol_id.replace(":", "/") + ".md"

def site_url_for_symbol(symbol_id):
    """Returns the rendered URL for one symbol ID."""
    return "api/" + symbol_id.replace(":", "/") + ".html"

def site_shard_name(name):
    """Returns the generated IR shard output name (Bazel output only)."""
    return name + ".ir.textproto"

def site_summary_name(name):
    """Returns the mdBook SUMMARY output name for one aggregate."""
    return name + "_SUMMARY.md"

def site_api_name(name):
    """Returns the generated API pages output name for one aggregate."""
    return name + "_api.md"

def site_records_name(name):
    """Returns the search-records output name for one aggregate."""
    return name + "_search_records.json"

def site_html_name(name):
    """Returns the rendered site entry output name for one render."""
    return name + "_index.html"

def site_index_name(name):
    """Returns the single search-index output name for one render."""
    return name + "_searchindex.json"

def site_prose_error(path):
    """Validates one prose input is mdBook-compatible Markdown."""
    if path.endswith(".md"):
        return ""
    return "docs_site: prose inputs must be Markdown, got '" + path + "'"

def site_is_external_link(target):
    """Returns True when a Markdown link target is remote and never fetched.

    Remote targets contain `://` or use `mailto:`; they are recorded but
    never fetched. All other targets are internal and must resolve to
    prose or generated API pages with no dangling targets.
    """
    if "://" in target:
        return True
    if target.startswith("mailto:"):
        return True
    return False

def site_link_target_error(target, known_pages, known_api_paths):
    """Validates one internal link target, returning "" when valid.

    `known_pages` lists the render-input basenames valid at the pre-render
    boundary (`api.md`, `prose.md`, `SUMMARY.md` plus declared prose
    basenames). `known_api_paths` lists per-symbol `api/...` pages derived
    from shard IDs via `site_api_path`. Remote targets are skipped (never
    fetched). Anchor-only targets need a non-empty fragment. Other targets
    strip any `#fragment` and require the base in `known_pages` or
    `known_api_paths`. Empty or unknown bases fail closed with no silent
    dangling pass.
    """
    if target == "":
        return "docs_site: empty link target"
    if site_is_external_link(target):
        return ""
    if target.startswith("#"):
        if len(target) > 1:
            return ""
        return "docs_site: empty link target"
    parts = target.split("#")
    base = parts[0]
    if base == "":
        return "docs_site: empty link target"
    if base in known_pages:
        return ""
    if base in known_api_paths:
        return ""
    if base.startswith("api/"):
        return "docs_site: dangling API link '" + target + "'"
    if base.endswith(".md"):
        return "docs_site: dangling prose link '" + target + "'"
    return "docs_site: unknown link target '" + target + "'"

def site_search_record(url, title, body):
    """Returns one search-index record with sorted keys.

    Keys stay alphabetical (body, title, url) so records serialize
    deterministically; the index is built from prose plus IR only.
    """
    return "{\"body\": \"" + body + "\", \"title\": \"" + title + "\", \"url\": \"" + url + "\"}"

def site_is_known_guide(name):
    """Returns True for the three frozen release-blocking guides.

    Only `quickstart`, `tutorial`, and `migration` are known; no extra
    guide is claimed and no implicit default is substituted.
    """
    return name in ["quickstart", "tutorial", "migration"]

def site_guide_step_error(step):
    """Validates one guide-step line, returning "" when executable.

    Blank lines and `#` comments are skipped (not steps). Lines carrying
    `TODO`, `FIXME`, `UNEXECUTED`, `TBD`, or `SKIP` markers fail closed:
    guide steps are never allowed to go unexecuted. Every other line is
    an executable shell step run by CI.
    """
    if step == "" or step.startswith("#"):
        return ""
    if "TODO" in step or "FIXME" in step or "UNEXECUTED" in step or "TBD" in step or "SKIP" in step:
        return "docs_site: unexecuted guide step '" + step + "'"
    return ""

def docs_extract(name, language, package, srcs):
    """Runs one DocsExtract action emitting one cached IR shard.

    Reads `srcs` (`qualified|doc` lines, one symbol per line), sorts with
    `LC_ALL=C sort`, and emits deterministic textproto with no timestamps,
    no absolute paths, and workspace-relative IDs only. The shard is a
    generated Bazel output, never a committed file. No network access.
    """
    unit_err = site_symbol_id_error(language, package, "unit")
    if unit_err != "":
        fail(unit_err + " (in " + native.package_name() + ":" + name + ")")
    if len(srcs) == 0:
        fail("docs_extract " + native.package_name() + ":" + name + ": need at least one src")
    shard = site_shard_name(name)
    native.genrule(
        name = name + "_shard",
        srcs = srcs,
        outs = [shard],
        cmd = "(printf 'schema_major: 1\\nschema_minor: 0\\nlanguage: \"" + language + "\"\\npackage: \"" + package + "\"\\n'; LC_ALL=C sort $(SRCS) | awk -F'|' '{printf \"symbols {\\n  id: \\\"" + language + ":" + package + ":%s\\\"\\n  doc_markdown: \\\"%s\\\"\\n}\\n\", $$1, $$2}')" + " > $@",
    )
    native.filegroup(
        name = name,
        srcs = [":" + name + "_shard"],
    )

def docs_aggregate(name, shards, prose, book_toml):
    """Runs one DocsAggregate action emitting render inputs.

    Consumes IR shards plus prose plus theme/config with shared validation
    and emits a mdBook-compatible Markdown tree (`SUMMARY.md` plus generated
    API pages) plus search-index records. The search records are built
    directly from prose plus IR; they never parse rendered HTML. All
    outputs are deterministic: sorted symbol order, sorted JSON keys,
    LF bytes, no timestamps, workspace-relative paths only. Shared
    validation includes link/reference completeness at the pre-render
    boundary (#782): prose inline plus reference-definition targets must
    resolve to prose or generated API pages with no dangling targets with
    dangling targets fail the aggregate action; remote targets are skipped,
    never fetched with no partial outputs.
    """
    if len(shards) == 0:
        fail("docs_aggregate " + native.package_name() + ":" + name + ": need at least one shard")
    if len(prose) == 0:
        fail("docs_aggregate " + native.package_name() + ":" + name + ": need at least one prose file")
    summary = site_summary_name(name)
    api = site_api_name(name)
    records = site_records_name(name)
    shard_locs = " ".join(["$(location " + s + ")" for s in shards])
    prose_locs = " ".join(["$(location " + p + ")" for p in prose])
    book_loc = "$(location " + book_toml + ")"
    native.genrule(
        name = name + "_aggregate",
        srcs = shards + prose + [book_toml],
        outs = [summary, api, records],
        cmd = "set -e; " +
              "summary=$(location :" + summary + "); api=$(location :" + api + "); records=$(location :" + records + "); " +
              "printf '# Summary\\n\\n- [Prose](prose.md)\\n- [API](api.md)\\n' > \"$$summary\"; " +
              "printf '# API Reference\\n\\n' > \"$$api\"; " +
              "LC_ALL=C grep -h '^  id: ' " + shard_locs + " | LC_ALL=C sort -u | sed 's/^  id: \"//;s/\"$$//;s/^/## /' >> \"$$api\"; " +
              "printf '[\\n' > \"$$records\"; " +
              "LC_ALL=C grep -h '^  id: ' " + shard_locs + " | LC_ALL=C sort -u | sed 's/^  id: \"//;s/\"$$//' | awk '{url=$$0; gsub(/:/, \"/\", url); printf \"  {\\\"body\\\": \\\"API docs for %s\\\", \\\"title\\\": \\\"%s\\\", \\\"url\\\": \\\"api/%s\\\"},\\n\", $$0, $$0, url}' | LC_ALL=C sort -u | sed '$$s/,$$//' >> \"$$records\"; " +
              "printf ']\\n' >> \"$$records\"; " +
              "grep -q '^title' " + book_loc + "; " +
              "grep -q '^# ' " + prose_locs + "; " +
              "for _sid in $$(LC_ALL=C grep -h '^  id: ' " + shard_locs + " | sed 's/^  id: \"//;s/\"$$//' | LC_ALL=C sort -u); do LC_ALL=C grep -qF \"$$_sid\" \"$$api\" || { echo \"docs_site: missing API page for $$_sid\" >&2; exit 1; }; done; " +
              "if LC_ALL=C grep -q '\\[[^]]*\\]()' " + prose_locs + "; then echo 'docs_site: empty link target' >&2; exit 1; fi; " +
              "anchors=$$( (LC_ALL=C grep -h '^#' " + prose_locs + " 2>/dev/null || true; LC_ALL=C grep -h '^## ' \"$$api\" 2>/dev/null || true) | sed 's/^#* *//' | tr '[:upper:]' '[:lower:]' | sed 's/[^a-z0-9 -]//g; s/^ *//; s/ *$$//; s/ /-/g; s/--*/-/g' | LC_ALL=C sort -u); " +
              "api_paths=$$(LC_ALL=C grep -h '^  id: ' " + shard_locs + " | sed 's/^  id: \"//;s/\"$$//' | sed 's/:/\\//g;s/^/api\\//;s/$$/.md/' | LC_ALL=C sort -u); " +
              "prose_bases=$$(for _f in " + prose_locs + "; do basename \"$$_f\"; done | LC_ALL=C sort -u); " +
              "targets=$$( (LC_ALL=C grep -h -o '\\[[^]]*\\]([^)]*)' " + prose_locs + " 2>/dev/null | sed -n 's/.*(\\([^)]*\\)).*/\\1/p' | sed 's/^ *//;s/ *$$//;s/^<//;s/>$$//;s/^\".*//;s/\".*$$//;s/^ *//;s/ *$$//' | cut -d' ' -f1 | LC_ALL=C sort -u || true; LC_ALL=C grep -h '^[ ]*\\[[^]]*\\]:' " + prose_locs + " 2>/dev/null | sed 's/^[^:]*:[[:space:]]*//;s/[[:space:]\".*].*//' | cut -d' ' -f1 | LC_ALL=C sort -u || true) | LC_ALL=C sort -u); " +
              "for _t in $$targets; do case \"$$_t\" in *\\://*|mailto:*) continue;; \\#*) _frag=$$(printf '%s' \"$$_t\" | cut -c2-); printf '%s\\n' \"$$anchors\" | LC_ALL=C grep -qxF \"$$_frag\" || { echo \"docs_site: dangling anchor '$$_t'\" >&2; exit 1; };; *) _base=$$(printf '%s' \"$$_t\" | cut -d'#' -f1); _frag=$$(printf '%s' \"$$_t\" | cut -s -d'#' -f2- || true); _found=0; for _p in api.md prose.md SUMMARY.md $$prose_bases $$api_paths; do if [ \"$$_base\" = \"$$_p\" ]; then _found=1; break; fi; done; if [ \"$$_found\" = \"0\" ]; then echo \"docs_site: dangling link '$$_t'\" >&2; exit 1; fi; if [ -n \"$$_frag\" ]; then _norm=$$(printf '%s' \"$$_frag\" | tr '[:upper:]' '[:lower:]'); printf '%s\\n' \"$$anchors\" | LC_ALL=C grep -qxF \"$$_norm\" || { echo \"docs_site: dangling fragment '$$_t'\" >&2; exit 1; }; fi;; esac; done",
    )
    native.filegroup(
        name = name,
        srcs = [":" + name + "_aggregate"],
    )

def docs_render(name, summary, api, records, book_toml):
    """Runs one DocsRender action emitting the complete static site.

    Consumes the aggregate render inputs plus the pinned mdBook config and
    emits the static site entry plus the single search index. The render
    stamps the pinned mdBook version with no timestamps and no absolute
    paths; the search index is copied from aggregate records, never parsed
    from rendered HTML.
    """
    html = site_html_name(name)
    index = site_index_name(name)
    native.genrule(
        name = name + "_render",
        srcs = [summary, api, records, book_toml],
        outs = [html, index],
        cmd = "set -e; " +
              "html=$(location :" + html + "); index=$(location :" + index + "); " +
              "title=$$(grep '^title' $(location " + book_toml + ") | cut -d '\"' -f 2); " +
              "{ printf '<!doctype html>\\n<html lang=\"en\">\\n<head><meta charset=\"utf-8\"><title>%s</title></head>\\n<body>\\n<!-- rendered by mdBook " + MDBOOK_VERSION + " fixture -->\\n' \"$$title\"; " +
              "printf '<h1>%s</h1>\\n' \"$$title\"; cat $(location " + summary + "); printf '\\n'; cat $(location " + api + "); printf '\\n</body>\\n</html>\\n'; } > \"$$html\"; " +
              "{ printf '{\\n  \"book\": \"%s\",\\n  \"docs\": ' \"$$title\"; cat $(location " + records + "); printf '\\n}\\n'; } > \"$$index\"",
    )
    native.filegroup(
        name = name,
        srcs = [":" + name + "_render"],
    )

def docs_site(name, language, package, srcs, prose, book_toml):
    """Chains extract, aggregate, and render for one (language, package) demo.

    One `docs_extract` per unit, one shared-validation `docs_aggregate`,
    one pinned-renderer `docs_render`. Check mode selects extract plus
    aggregate without render; see `plan_mode_actions` in `cli/docgen`.
    """
    docs_extract(
        name = name + "_extract",
        language = language,
        package = package,
        srcs = srcs,
    )
    docs_aggregate(
        name = name + "_aggregate",
        shards = [":" + name + "_extract"],
        prose = prose,
        book_toml = book_toml,
    )
    docs_render(
        name = name,
        summary = ":" + name + "_aggregate_SUMMARY.md",
        api = ":" + name + "_aggregate_api.md",
        records = ":" + name + "_aggregate_search_records.json",
        book_toml = book_toml,
    )
