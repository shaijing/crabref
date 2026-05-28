# Crabref — paper reference management tool
# Database URL is read from DATABASE_URL in .env
# Run `just` or `just help` for available commands.

default: check

# === Build ===

# Build the library and CLI binary
build:
    cargo build

# Build release binary
release:
    cargo build --release

# Type-check without full build
check:
    cargo check

# === Test ===

# Run all tests
test:
    cargo test

# Run tests with verbose output
test-verbose:
    cargo test -- --nocapture

# Run a specific test
test-name NAME:
    cargo test {{NAME}} -- --nocapture

# === Code Quality ===

# Run clippy lints
clippy:
    cargo clippy -- -D warnings

# Format the code
fmt:
    cargo fmt

# === Database ===

# Push database schema (create tables)
db-push:
    cargo run -- push-schema

# Reset database (drop and recreate schema)
db-reset:
    cargo run -- push-schema

# === CLI ===

# Show CLI help
help:
    cargo run -- --help

# Show config
config:
    cargo run -- config

# List sources
sources:
    cargo run -- sources

# List local papers
list LIMIT="10":
    cargo run -- list -l {{LIMIT}}

# Count papers
count:
    cargo run -- count

# Search across all sources
search QUERY LIMIT="5":
    cargo run -- search -q "{{QUERY}}" -l {{LIMIT}}

# Search arXiv
arxiv-search QUERY LIMIT="5":
    cargo run -- arxiv-search -q "{{QUERY}}" -l {{LIMIT}}

# Search Semantic Scholar
ss-search QUERY LIMIT="5":
    cargo run -- ss-search -q "{{QUERY}}" -l {{LIMIT}}

# Search OpenAlex
oa-search QUERY LIMIT="5":
    cargo run -- oa-search -q "{{QUERY}}" -l {{LIMIT}}

# Search database
db-search QUERY:
    cargo run -- db-search -q "{{QUERY}}"

# Search database by author
db-search-author QUERY:
    cargo run -- db-search -q "{{QUERY}}" -f author

# Get paper by identifier
get ID:
    cargo run -- get -i "{{ID}}"

# Get paper by arXiv ID
get-arxiv ID:
    cargo run -- get-arxiv -i "{{ID}}"

# Get paper by SS ID
get-ss ID:
    cargo run -- get-ss -i "{{ID}}"

# Fetch and save paper
fetch ID:
    cargo run -- fetch -i "{{ID}}"

# Get citations for a paper
citations PAPER_ID LIMIT="20":
    cargo run -- citations -i "{{PAPER_ID}}" -l {{LIMIT}}

# Get references for a paper
references PAPER_ID LIMIT="20":
    cargo run -- references -i "{{PAPER_ID}}" -l {{LIMIT}}

# Export papers to BibTeX
export-bibtex:
    cargo run -- export -f bibtex

# Export papers to JSON
export-json:
    cargo run -- export -f json

# Export papers to Markdown
export-markdown:
    cargo run -- export -f markdown

# Export filtered papers
export-query QUERY FORMAT="bibtex":
    cargo run -- export -f {{FORMAT}} -q "{{QUERY}}"

# Export to file
export-file FORMAT="bibtex" OUTPUT="papers.bib":
    cargo run -- export -f {{FORMAT}} -o "{{OUTPUT}}"

# Citation statistics
citation-stats:
    cargo run -- citation-stats

# Citation graph (DOT format)
citation-graph:
    cargo run -- citation-graph -f dot

# Citation graph as JSON
citation-graph-json:
    cargo run -- citation-graph -f json

# Export citation graph to file
citation-graph-file FORMAT="dot" OUTPUT="citation_graph.dot":
    cargo run -- citation-graph -f {{FORMAT}} -o "{{OUTPUT}}"

# Sync citation counts
sync-citations BATCH="20":
    cargo run -- sync-citations -b {{BATCH}}

# Download PDF for a paper
download ID:
    cargo run -- download -i "{{ID}}"

# Save a paper manually
save TITLE ARXIV_ID="" SS_ID="":
    cargo run -- save -t "{{TITLE}}" -a "{{ARXIV_ID}}" -s "{{SS_ID}}"

# Delete a paper by DB ID
delete ID:
    cargo run -- delete -i {{ID}}

# Update a paper from source
update ID:
    cargo run -- update -i {{ID}}

# Recent papers (default: last 7 days)
recent DAYS="7":
    cargo run -- recent -d {{DAYS}}

# Run with API key
with-api-key KEY:
    cargo run -- --ss-api-key "{{KEY}}" ss-search -q "machine learning"

# Run with proxy
with-proxy PROXY_URL:
    cargo run -- --proxy "{{PROXY_URL}}" sources

# === Development ===

# Watch for changes and rebuild
watch:
    cargo watch -x check

# Watch and run tests
watch-test:
    cargo watch -x test

# Clean build artifacts
clean:
    cargo clean

# Full CI check
ci: check test clippy
    @echo "All checks passed!"