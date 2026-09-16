// Deobfuscated from r.class (extends y).
//
// Per-level race setup (the `.001` record the career points at).
// Layout verified against tools/kora/formats.py RaceConfig and both
// campaign files (all 47 records decode to in-range values).
// UI bodies TODO.
//
// The `.001` file is a sequence of 6 byte blocks padded to the
// offsets in the `.000` extras table; the game seeks to values[2],
// reads one block, and stops. Field order depends on the mode:
//
//   mode 0/4 (cu.b): laps, theme, flag, car, param, opponents
//   mode 1   (r.b):  theme, flag, car, param, opponents (one lap)
//   mode 2/6 (dk.b): theme, flag, laps, car, i32 timeLimit
//   mode 3   (cd.b): theme, flag, car, param, opponents (laps := opponents)
//   mode 5   (bv.b): theme, flag, laps, car, i32 timeLimit
//
// theme selects the tile/texture variant (bm/bp compare with 3, dk
// forces 3 for 8a.map); car is the player car index (>= 50 marks the
// deluxe time attack entries); modes 2/5/6 are solo clocks, the rest
// race against opponents. Laps, theme, opponents, and the
// TIME CHASE / SLIDESHOW / SPECIAL clock modes line up with ui.txt
// labels 202-208 and car stats 127-130.
//
// Field heads include a:Lbs (the built track), a:Lbt (the
// controller), a:B, b:[I, s/e/f/g/h/i/j/k/l/m/n/o:I counters,
// a:Le, a:Z/f:Z flags, a:[Ljava/lang/String. Full inventory in
// /tmp/clsinfo output; method bodies (b/c/d/e/f/g/h/i/j/k/m/n/o/p,
// a(F)V, b(F)V, graphics painters) TODO.

public class RaceConfig {

    public int mode;
    public int laps;
    public int theme;
    public boolean flag;
    public int car;
    public int opponents;
    public Integer param;      // null on clock modes
    public Integer timeLimit;  // null on race modes

    // TODO(obfuscated): r field renames + screen method bodies.
}
