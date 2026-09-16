// Deobfuscated from cl.class (extends ca).
//
// One car on the track (player or AI opponent). Constructor takes the
// track, so physics and rendering both reference GameTrack.
// Inventory below is complete (73 fields, 63 methods); bodies are
// TODO except where noted. This is the next file to walk for the
// floating rail/rock question if it turns out to be camera or car
// height rather than placement (the port holds cars to
// surface_height via SurfaceGrid::support_height with a nose sample
// and a 0.2 lead cap; confirm cl does the same per-frame mesh
// sampling here).
//
// Field map (first 30 of 73, original names kept where role unknown):
//   cl.a:I, cl.b:I, cl.a:Ldi, cl.a:Lx, cl.d:F, cl.e:F, cl.a:Lat,
//   cl.h:Z, cl.a:Z, cl.i:Z, cl.e:Lbz, cl.a:Lbz, cl.b:Lbz, cl.f:Lbz,
//   cl.g:Lbz, cl.h:Lbz, cl.a:Lbj, cl.j:Z, cl.f:F, cl.g:F, cl.h:F,
//   cl.i:F, cl.a:Lz, cl.b:Lz, cl.g:I, cl.h:I, cl.c:I, cl.i:I, cl.j:I,
//   ... +43 more (see /tmp/clsinfo output for the rest).
// at field (cl.a:Lat) is the car body mesh; di/x/bz/bj/z are vector
// and helper types shared with bs/bm height sampling.

public class RaceCar {

    // TODO(obfuscated): assign readable names field by field while
    // walking cl.<init>(Lcf;InputStream;ILbs;I)V. The third int and
    // the trailing int are probably car index / opponent flag; the
    // InputStream is the .car descriptor (model, low model ignored,
    // texture, name, 4 stat bytes, 39 unread bytes).

    public RaceCar(Object assetManager, java.io.InputStream carDesc,
                   int unknownA, GameTrack track, int unknownB) {
        // TODO(obfuscated): walk the constructor.
    }

    // Lifecycle (bodies TODO):
    //   cl.c()V, cl.d()V, cl.e()V, cl.a(Lcf;)V, cl.a(Lbq;)V,
    //   cl.b(Lbq;)V, cl.a(Lcf;Z)V, cl.c(Lbq;)V
    //
    // Controls (float setters, bodies TODO):
    //   cl.f(F)V, cl.g(F)V, cl.n(F)V, cl.j(F)V, cl.o(F)V, cl.h(F)V,
    //   cl.i(F)V, cl.k(F)V, cl.a(F)V, cl.b(F)V, cl.c(F)V, cl.l(F)V,
    //   cl.d(F)V, cl.e(F)V, cl.m(F)V
    //
    // Queries:
    //   cl.f()V, cl.g()V, cl.h()V, cl.a(F)I, cl.i()V,
    //   cl.a(Lct;Lbz;FFLbz;)V, cl.a(Lcl;Z)Z, cl.b()Z, cl.a()V,
    //   cl.a()I, cl.b()I, cl.a()Ldi, cl.a(FFFLr;)V, cl.a()Lx,
    //   cl.a()Lbz, cl.b()Lbz, ... +23 more
    //
    // The (FFFLr;)V overload takes the race config (r) and is likely
    // the per-frame car update (steering/throttle/physics). Walk this
    // first when doing physics.
}
