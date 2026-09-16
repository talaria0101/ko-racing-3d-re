// Deobfuscated from bt.class (extends y).
//
// Race controller: owns the track, the cars, and the camera rigs for
// one race. Inventory complete (29 fields, 25 methods); bodies TODO.
// This is the class that calls
// bs.a/b(Lbq;Lj;[Lcl;III) and bm.a(Lbq;[Lcl;Lj;) with the car list,
// so the floating rail/rock question ends here if placement is
// right but the per-frame camera or car list passed to the track is
// wrong.
//
// Field map (original -> guess, TODO to confirm):
//   bt.a:Laj ?, bt.a:Lcl player?, bt.a:Lbs track, bt.a:Lj camA,
//   bt.b:Lj camB, bt.c:Lj camC, bt.a:Lcr ?, bt.a:Laa ?, bt.a:Ll ?,
//   bt.a:[Lj ?, bt.a:I ?, bt.a:J ?, bt.b:I, bt.c:I, bt.a:[Z ?,
//   bt.a:[Lcl opponents?, bt.a:F ?, bt.a:Z ?, bt.b:Z, bt.c:Z,
//   bt.d:I, bt.e:I, bt.d:Z, bt.a:Lr config, bt.a:[F ?, bt.a:Lq ?,
//   bt.e:Z, bt.b:F, bt.c:F
//
// Constructors:
//   bt.<init>(Lcv;Lr;)V (new race from config)
//   bt.<init>(Lcv;Lr;InputStream;)V (restore?)

public class RaceController {

    public GameTrack track;   // bt.a:Lbs
    public RaceCar player;    // guess bt.a:Lcl
    public RaceCar[] cars;    // guess bt.a:[Lcl
    public RaceConfig config; // bt.a:Lr

    public RaceController(Object canvas, RaceConfig config) {
        // TODO(obfuscated).
    }

    // Loop (bodies TODO):
    //   bt.e()V, bt.s()V, bt.a()V, bt.j()V,
    //   bt.a(Graphics)V, bt.a()Z, bt.a(F)V, bt.b()Z, bt.a()F,
    //   bt.b(F)V, bt.c()Z, bt.a(I)Lcl, bt.c(F)V, bt.a(I)V,
    //   bt.c(I)V, bt.d()V, bt.h()V, bt.g()V,
    //   bt.a(II)Z, bt.a(II)V, bt.b(II)V
}
