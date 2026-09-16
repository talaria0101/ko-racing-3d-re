// Deobfuscated from at.class (extends am) plus de/cf material paths.
//
// Mesh file to M3G: reads the custom byte sized mesh, builds a
// VertexArray / VertexBuffer / TriangleStripArray / Mesh with an
// Appearance. Header floats and the 128 offset verified; car squash
// and appearance branches partly TODO.
//
// Mesh file layout:
//   u8[3] zero magic
//   str positionScale (float as text, Float.valueOf)
//   u8 reserved, str positionBias, u8 reserved,
//   str texcoordScale, u8 reserved, str texcoordBias
//   u8 vertexCount
//   repeat: i8 x,y,z + i8 u,v (SIGNED bytes in Java byte[]; M3G
//     component type 1 = BYTE decodes them signed)
//   u8 stripCount + strip lengths, u16 indexCount + u8 indices
// Decoded (cf. m3g_lwjgl VertexBuffer.getVertex/getTexVertex):
//   pos = raw * posScale + (128 * posScale + posBias)
//   uv  = raw * uvScale  + (128 * uvScale  + uvBias)
// The 128 term shifts the signed range back onto 0..1 for cars and
// tile atlas rectangles. Reading the bytes unsigned adds 256*scale
// to components above 127 (a full wrap, near 1.0 since scales sit
// near 1/256) and smears liveries; the ab2fd92 / car livery history
// covers this. Genuine wraps exist (zdzn spans u 0..8) because M3G
// repeats lookups; a clamping viewer shows those wrong.
//
// at floats seen in the constant pool: 128.0 plus car scales 0.86,
// 0.9, 0.95, 0.96 family. The car body keeps natural size with a
// 0.96/0.96/0.9 squash (scene.rs build_car); tiles/detail/objects
// get the 7.01 node scale instead (see M3GNode).
//
// Appearance (at.a(Lcf,...) overloads, cf.a texture loads):
// PolygonMode culling 160 (CULL_BACK), local camera lighting off,
// perspective correction from al.a, shading 165, two sided lighting
// off; CompositingMode blending 64 with depth write/test off only on
// the transparent path (al.g). Texture filtering nearest (al.d =
// 210). Bodies TODO beyond the constant pool evidence.

public class MeshLoader extends M3GNode {

    public float positionScale; // at.a
    public float positionBias;
    public float texcoordScale; // at.b
    public float texcoordBias;

    public MeshLoader(String model, Object appearance, boolean shared) {
        // TODO(obfuscated): at.<init>(String, Appearance, boolean)V.
    }

    // at.a(Lcf,String,IZ), at.a(Lcf,String,IZZ),
    // at.a(Lcf,String,ZZI): appearance builders. TODO.
    // at.a(String,[B)[B, at.a(String,Appearance,Z)V,
    // at.a(Texture2D)V, at.b(Texture2D)V, at.b()V, at.c()V: TODO.
}
