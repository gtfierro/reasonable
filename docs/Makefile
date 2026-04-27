# Minimal Makefile for Sphinx documentation
SPHINXBUILD   ?= sphinx-build
SOURCEDIR     = .
BUILDDIR      = _build

.PHONY: help clean html llms

help:
	@$(SPHINXBUILD) -M help "$(SOURCEDIR)" "$(BUILDDIR)"

html:
	@$(SPHINXBUILD) -M html "$(SOURCEDIR)" "$(BUILDDIR)"

# Produce a single concatenated text file for LLM ingestion.
llms:
	@$(SPHINXBUILD) -M text "$(SOURCEDIR)" "$(BUILDDIR)"
	@find "$(BUILDDIR)/text" -name '*.txt' -print | sort | xargs cat > "$(BUILDDIR)/llms.txt"
	@echo "Wrote $(BUILDDIR)/llms.txt"

clean:
	rm -rf "$(BUILDDIR)"
