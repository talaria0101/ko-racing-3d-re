// Deobfuscated from bh.class (extends y) and bd.class (extends bh).
//
// Race screens. bh is the shared base (58 fields, 34 methods); bd is
// the concrete race stage (106 fields, 36 methods). Inventories
// complete; state machine bodies TODO. The single letter method names
// (a, b, c, h, j, s, k, m, n, o, p, q, r, t, u, v, C-I, etc.) are the
// obfuscated lifecycle and event handlers; do not rename them until
// each is walked against cv (the GameCanvas frame switch).
//
// bh field heads: a:Lcm, a:Lbi, b:Lbi, d:Lbi, e:Lbi, f:Lbi, b:Lcm,
// g:Lbi, h:Lbi, i:Lbi, c:Lbi, g:Lcm, a:Ldb, j:Lbi, h:Lcm, a:Lcp,
// b:Lcp, b:Ldb, c:Ldb, d:Ldb, e:Ldb, f:Ldb, g:Ldb, c:Lcp, d:Lcp,
// c:Lcm, a:Lao, h:Ldb, a:Lbk, i:Lcm, ... +28 more.
// bd adds: a:F, a:[Lcq, a:Lcq, f:I, a:Background, a:Transform, d:Z,
// g:I, b:F, c:F, d:F, a:Image, d:Lbi and more image/font slots,
// g:Lcm, h:Lcm, a:Lch, i:Lcm, ... +76 more (background, minimap,
// countdown, position, lap, timing).
//
// If you walk these next, start from cv.run() (the frame loop) into
// bh/bd.a(Graphics) and the key handlers a(II)Z / a(II)V / b(II)V.

public class RaceBase {
    // TODO(obfuscated): bh field renames + method bodies.
}

public class RaceStage extends RaceBase {
    // TODO(obfuscated): bd field renames + method bodies.
}

/*
 * PAINT ORDER (bd, verified heads).
 *
 * bd.a(Graphics) paints the 3D world through bq (Background colour
 * clear, track flood, cars) and bd.b(Graphics) layers the UI widgets
 * (cm/bi: lap, position, time, minimap, pause icon top-right via
 * drawImage at clipWidth - imgWidth - 3). bd.m()V builds the widget
 * set and adds the sky-strip widget (bi id 10000) when the al.a sky
 * path from al.q(theme) is non-empty. bh/r/u/co/dm/aj share the
 * colour-clear + widget pattern with per-screen widget sets.
 */
