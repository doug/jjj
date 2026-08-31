# Not seeded

`seed.sh` copies `fixture/`, `score.sh`, `verify.sh` and `groundtruth.sh` into
the agents' workbench. It does **not** copy this directory, and must not: it
holds the answers.

`optimised_pipeline.py` is a reference implementation with all five levers
pulled and the four passes fused. It exists to answer one question the seeded
ground-truth check cannot ask without giving the game away — **is the scoring
ceiling actually out of reach?**

That question is what the first synthetic target got wrong. Its full-marks floor
was attainable, six agents attained it in ten minutes, and the A/B trial run on
it could not distinguish its two arms because both finished at 100.
