"""Generate bloat the Go linker cannot eliminate, for the ground-truth check."""
import os
import pathlib

d = pathlib.Path("examples/counter")
(d / "bloat_blob.bin").write_bytes(os.urandom(4_000_000))
(d / "bloat_gen.go").write_text(
    'package main\n\nimport _ "embed"\n\n'
    '//go:embed bloat_blob.bin\nvar bloatBlob []byte\n\n'
    'func init() { bloatSink = len(bloatBlob) }\n\nvar bloatSink int\n'
)
