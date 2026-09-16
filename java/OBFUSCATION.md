# Obfuscated -> readable name map

Generated files (`Obf*.java`, `Gen*.java`, `KORa.java`,
`VservManager.java`) come from `tools/jdeob.py` and must
not be hand edited. The curated race-path files
(e.g. `GameTrack.java` for `bs`) are the reviewed layer;
the matching `Gen*.java` file is the raw emitter output
kept beside it for review.

| original | file | role |
|---|---|---|
| `b` | `GenObfB.java` | ()I; ()Ljava/lang/Class;; ()Ljava/lang/String;; ()V; java/io |
