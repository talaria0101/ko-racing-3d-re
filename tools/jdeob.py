"""Batch deobfuscator: every obfuscated game class becomes one readable
``ObfXx.java`` file with renamed references, typed locals and labelled
``goto`` control flow.  Faithful over pretty: control flow is emitted
exactly as the bytecode branches (labels plus ``goto``), so a reviewer
can follow every jump without trusting a structure recovery pass.

Usage:
    python3 tools/jdeob.py            # emit all classes into java/
    python3 tools/jdeob.py aq         # emit one class (obfuscated name)

Inputs: x/*.class (the shipped jar extraction) plus
``tools/jdeob_names.tsv`` (obfuscated -> readable name overrides).
Outputs: java/Obf*.java (generated, do not hand edit) and
java/OBFUSCATION.md (the name map plus per-class role notes).

Only the standard library is used.  Imported as a module it exposes
``decompile_class`` for the review passes.
"""

import importlib.util as _ilu
import math
import os
import struct
import sys

_HERE = os.path.dirname(os.path.abspath(__file__))
_REPO = os.path.dirname(_HERE)
_XDIR = os.path.join(_REPO, 'x')
_JDIR = os.path.join(_REPO, 'java')

_spec = _ilu.spec_from_file_location(
    'jdis', os.path.join(_HERE, 'jdis.py'))
jdis = _ilu.module_from_spec(_spec)
_spec.loader.exec_module(jdis)

# ---------------------------------------------------------------------------
# opcode table: opcode -> (name, operand kind)
# operand kinds: - none, B u8, b s8, H u16, h s16, J branch s16,
#   I invokeinterface, M multianewarray, W wide, TS tableswitch,
#   LS lookupswitch, N newarray
# ---------------------------------------------------------------------------

_OPS = {}


def _op(code, name, kind=''):
    _OPS[code] = (name, kind)


for _i in range(6):
    _op(0x02 + _i, 'iconst_m1' if _i == 0 else 'iconst_%d' % (_i - 1))
for _i in range(4):
    _op(0x1a + _i, 'iload_%d' % _i)
    _op(0x22 + _i, 'fload_%d' % _i)
    _op(0x24 + _i, 'dload_%d' % _i) if False else None
    _op(0x1e + _i, 'lload_%d' % _i)
    _op(0x2a + _i, 'aload_%d' % _i)
    _op(0x3b + _i, 'istore_%d' % _i)
    _op(0x43 + _i, 'fstore_%d' % _i)
    _op(0x47 + _i, 'dstore_%d' % _i) if False else None
    _op(0x3f + _i, 'lstore_%d' % _i)
    _op(0x4b + _i, 'astore_%d' % _i)
_op(0x26, 'dload_0')
_op(0x27, 'dload_1')
_op(0x28, 'dload_2')
_op(0x29, 'dload_3')
_op(0x47, 'dstore_0')
_op(0x48, 'dstore_1')
_op(0x49, 'dstore_2')
_op(0x4a, 'dstore_3')
_op(0x2e + 0, 'iaload')
_op(0x2f, 'laload')
_op(0x30, 'faload')
_op(0x31, 'daload')
_op(0x32, 'aaload')
_op(0x33, 'baload')
_op(0x34, 'caload')
_op(0x35, 'saload')
_op(0x4f, 'iastore')
_op(0x50, 'lastore')
_op(0x51, 'fastore')
_op(0x52, 'dastore')
_op(0x53, 'aastore')
_op(0x54, 'bastore')
_op(0x55, 'castore')
_op(0x56, 'sastore')
_op(0x57, 'pop')
_op(0x58, 'pop2')
_op(0x59, 'dup')
_op(0x5a, 'dup_x1')
_op(0x5b, 'dup_x2')
_op(0x5c, 'dup2')
_op(0x5d, 'dup2_x1')
_op(0x5e, 'dup2_x2')
_op(0x5f, 'swap')
_op(0x60, 'iadd')
_op(0x61, 'ladd')
_op(0x62, 'fadd')
_op(0x63, 'dadd')
_op(0x64, 'isub')
_op(0x65, 'lsub')
_op(0x66, 'fsub')
_op(0x67, 'dsub')
_op(0x68, 'imul')
_op(0x69, 'lmul')
_op(0x6a, 'fmul')
_op(0x6b, 'dmul')
_op(0x6c, 'idiv')
_op(0x6d, 'ldiv')
_op(0x6e, 'fdiv')
_op(0x6f, 'ddiv')
_op(0x70, 'irem')
_op(0x71, 'lrem')
_op(0x72, 'frem')
_op(0x73, 'drem')
_op(0x74, 'ineg')
_op(0x75, 'lneg')
_op(0x76, 'fneg')
_op(0x77, 'dneg')
_op(0x78, 'ishl')
_op(0x79, 'lshl')
_op(0x7a, 'ishr')
_op(0x7b, 'lshr')
_op(0x7c, 'iushr')
_op(0x7d, 'lushr')
_op(0x7e, 'iand')
_op(0x7f, 'land')
_op(0x80, 'ior')
_op(0x81, 'lor')
_op(0x82, 'ixor')
_op(0x83, 'lxor')
_op(0x84, 'iinc', 'BB')
_op(0x85, 'i2l')
_op(0x86, 'i2f')
_op(0x87, 'i2d')
_op(0x88, 'l2i')
_op(0x89, 'l2f')
_op(0x8a, 'l2d')
_op(0x8b, 'f2i')
_op(0x8c, 'f2l')
_op(0x8d, 'f2d')
_op(0x8e, 'd2i')
_op(0x8f, 'd2l')
_op(0x8f + 1, 'd2f')
_op(0x91, 'i2b')
_op(0x92, 'i2c')
_op(0x93, 'i2s')
_op(0x94, 'lcmp')
_op(0x95, 'fcmpg')
_op(0x96, 'fcmpl')
_op(0x97, 'dcmpg')
_op(0x98, 'dcmpl')
for _c, _n in [(0x99, 'ifeq'), (0x9a, 'ifne'), (0x9b, 'iflt'),
               (0x9c, 'ifge'), (0x9d, 'ifgt'), (0x9e, 'ifle'),
               (0x9f, 'if_icmpeq'), (0xa0, 'if_icmpne'),
               (0xa1, 'if_icmplt'), (0xa2, 'if_icmpge'),
               (0xa3, 'if_icmpgt'), (0xa4, 'if_icmple'),
               (0xa5, 'if_acmpeq'), (0xa6, 'if_acmpne'),
               (0xc6, 'ifnull'), (0xc7, 'ifnonnull')]:
    _op(_c, _n, 'J')
_op(0xa7, 'goto', 'J')
_op(0xa8, 'jsr', 'J')
_op(0xa9, 'ret', 'B')
_op(0xaa, 'tableswitch', 'TS')
_op(0xab, 'lookupswitch', 'LS')
_op(0xac, 'ireturn')
_op(0xad, 'lreturn')
_op(0xae, 'freturn')
_op(0xaf, 'dreturn')
_op(0xb0, 'areturn')
_op(0xb1, 'return')
_op(0xb2, 'getstatic', 'H')
_op(0xb3, 'putstatic', 'H')
_op(0xb4, 'getfield', 'H')
_op(0xb5, 'putfield', 'H')
_op(0xb6, 'invokevirtual', 'H')
_op(0xb7, 'invokespecial', 'H')
_op(0xb8, 'invokestatic', 'H')
_op(0xb9, 'invokeinterface', 'I')
_op(0xbb, 'new', 'H')
_op(0xbc, 'newarray', 'B')
_op(0xbd, 'anewarray', 'H')
_op(0xbe, 'arraylength')
_op(0xbf, 'athrow')
_op(0xc0, 'checkcast', 'H')
_op(0xc1, 'instanceof', 'H')
_op(0xc2, 'monitorenter')
_op(0xc3, 'monitorexit')
_op(0xc4, 'wide', 'W')
_op(0xc5, 'multianewarray', 'M')
_op(0x10, 'bipush', 'b')
_op(0x11, 'sipush', 'h')
_op(0x12, 'ldc', 'B')
_op(0x13, 'ldc_w', 'H')
_op(0x14, 'ldc2_w', 'H')
_op(0x0b, 'fconst_0')
_op(0x0c, 'fconst_1')
_op(0x0d, 'fconst_2')
_op(0x0e, 'dconst_0')
_op(0x0f, 'dconst_1')
_op(0x03, 'iconst_0')
_op(0x04, 'iconst_1')
_op(0x05, 'iconst_2')
_op(0x06, 'iconst_3')
_op(0x07, 'iconst_4')
_op(0x08, 'iconst_5')
_op(0x09, 'lconst_0')
_op(0x0a, 'lconst_1')
_op(0x15, 'iload', 'B')
_op(0x16, 'lload', 'B')
_op(0x17, 'fload', 'B')
_op(0x18, 'dload', 'B')
_op(0x19, 'aload', 'B')
_op(0x36, 'istore', 'B')
_op(0x37, 'lstore', 'B')
_op(0x38, 'fstore', 'B')
_op(0x39, 'dstore', 'B')
_op(0x3a, 'astore', 'B')
_op(0x00, 'nop')
_op(0x01, 'aconst_null')

_NEWARRAY = {4: 'boolean', 5: 'char', 6: 'float', 7: 'double',
             8: 'byte', 9: 'short', 10: 'int', 11: 'long'}


def decode(bc):
    """Split bytecode into (pc, name, operand) triples."""
    ins = []
    pc = 0
    n = len(bc)
    while pc < n:
        opc = bc[pc]
        if opc not in _OPS:
            ins.append((pc, 'UNKNOWN_0x%02x' % opc, None, 1))
            pc += 1
            continue
        name, kind = _OPS[opc]
        if kind == '':
            ins.append((pc, name, None, 1))
            pc += 1
        elif kind == 'B':
            ins.append((pc, name, bc[pc + 1], 2))
            pc += 2
        elif kind == 'b':
            v = bc[pc + 1]
            ins.append((pc, name, v - 256 if v > 127 else v, 2))
            pc += 2
        elif kind == 'H':
            v = struct.unpack('>H', bc[pc + 1:pc + 3])[0]
            ins.append((pc, name, v, 3))
            pc += 3
        elif kind == 'h':
            v = struct.unpack('>h', bc[pc + 1:pc + 3])[0]
            ins.append((pc, name, v, 3))
            pc += 3
        elif kind == 'J':
            v = struct.unpack('>h', bc[pc + 1:pc + 3])[0]
            ins.append((pc, name, pc + v, 3))
            pc += 3
        elif kind == 'BB':
            ins.append((pc, name, (bc[pc + 1], bc[pc + 2]), 3))
            pc += 3
        elif kind == 'I':
            v = struct.unpack('>H', bc[pc + 1:pc + 3])[0]
            ins.append((pc, name, (v, bc[pc + 3], bc[pc + 4]), 5))
            pc += 5
        elif kind == 'M':
            v = struct.unpack('>H', bc[pc + 1:pc + 3])[0]
            ins.append((pc, name, (v, bc[pc + 3]), 4))
            pc += 4
        elif kind == 'TS':
            pad = (4 - (pc + 1) % 4) % 4
            p = pc + 1 + pad
            default = pc + struct.unpack('>i', bc[p:p + 4])[0]
            lo = struct.unpack('>i', bc[p + 4:p + 8])[0]
            hi = struct.unpack('>i', bc[p + 8:p + 12])[0]
            cases = []
            for k in range(hi - lo + 1):
                off = struct.unpack('>i', bc[p + 12 + 4 * k:p + 16 + 4 * k])[0]
                cases.append((lo + k, pc + off))
            ins.append((pc, name, (default, cases), 1 + pad + 12 + 4 * len(cases)))
            pc += 1 + pad + 12 + 4 * len(cases)
        elif kind == 'LS':
            pad = (4 - (pc + 1) % 4) % 4
            p = pc + 1 + pad
            default = pc + struct.unpack('>i', bc[p:p + 4])[0]
            np = struct.unpack('>i', bc[p + 4:p + 8])[0]
            cases = []
            for k in range(np):
                key = struct.unpack('>i', bc[p + 8 + 8 * k:p + 12 + 8 * k])[0]
                off = struct.unpack('>i', bc[p + 12 + 8 * k:p + 16 + 8 * k])[0]
                cases.append((key, pc + off))
            ins.append((pc, name, (default, cases), 1 + pad + 8 + 8 * len(cases)))
            pc += 1 + pad + 8 + 8 * len(cases)
        elif kind == 'W':
            opc2 = bc[pc + 1]
            idx = struct.unpack('>H', bc[pc + 2:pc + 4])[0]
            if opc2 == 0x84:
                c = struct.unpack('>h', bc[pc + 4:pc + 6])[0]
                ins.append((pc, 'iinc', (idx, c), 6))
                pc += 6
            else:
                wname = {0x15: 'iload', 0x16: 'lload', 0x17: 'fload',
                         0x18: 'dload', 0x19: 'aload', 0x36: 'istore',
                         0x37: 'lstore', 0x38: 'fstore', 0x39: 'dstore',
                         0x3a: 'astore', 0xa9: 'ret'}.get(opc2, 'WIDE_0x%02x' % opc2)
                ins.append((pc, wname, idx, 4))
                pc += 4
        elif kind == 'N':
            ins.append((pc, name, bc[pc + 1], 2))
            pc += 2
        else:  # pragma: no cover
            ins.append((pc, 'BADKIND_' + name, None, 1))
            pc += 1
    return ins


# ---------------------------------------------------------------------------
# descriptors and names
# ---------------------------------------------------------------------------

def split_desc(desc):
    """Split a method descriptor into (param_types, return_type)."""
    assert desc[0] == '('
    depth = 0
    i = 1
    params = []
    cur = ''
    while True:
        c = desc[i]
        if c == ')':
            break
        cur += c
        if c == 'L':
            while desc[i] != ';':
                i += 1
                cur += desc[i]
            params.append(cur)
            cur = ''
        elif c == '[':
            pass
        else:
            if cur.startswith('[') or len(cur) == 1:
                params.append(cur)
                cur = ''
        i += 1
    return params, desc[i + 1:]


def java_type(desc, names):
    """A field/method descriptor becomes a Java type name."""
    dims = 0
    while desc.startswith('['):
        dims += 1
        desc = desc[1:]
    if desc == 'V':
        base = 'void'
    elif desc == 'I':
        base = 'int'
    elif desc == 'Z':
        base = 'boolean'
    elif desc == 'B':
        base = 'byte'
    elif desc == 'C':
        base = 'char'
    elif desc == 'S':
        base = 'short'
    elif desc == 'J':
        base = 'long'
    elif desc == 'F':
        base = 'float'
    elif desc == 'D':
        base = 'double'
    elif desc.startswith('L'):
        base = names.get(desc[1:-1].replace('/', '.'), desc[1:-1].split('/')[-1])
    else:
        base = desc
    return base + '[]' * dims


def readable_class(obf, names):
    dotted = obf.replace('/', '.')
    if dotted.startswith('java.') or dotted.startswith('javax.'):
        return dotted
    if dotted in names:
        return names[dotted]
    if dotted in ('KORa', 'VservManager'):
        names[dotted] = dotted
        return dotted
    tail = dotted.split('.')[-1]
    guess = 'Obf' + tail[0].upper() + tail[1:]
    names[dotted] = guess
    return guess


def load_names():
    names = {}
    path = os.path.join(_HERE, 'jdeob_names.tsv')
    if os.path.exists(path):
        for line in open(path):
            line = line.rstrip('\n')
            if not line or line.startswith('#'):
                continue
            obf, read = line.split('\t')[:2]
            names[obf] = read
    return names


# ---------------------------------------------------------------------------
# class file model
# ---------------------------------------------------------------------------

class Klass:
    def __init__(self, obf):
        self.obf = obf
        data, pos0, self.cp = jdis.load(os.path.join(_XDIR, obf + '.class'))
        pos = pos0 + 6
        n = struct.unpack('>H', data[pos:pos + 2])[0]
        pos += 2 + 2 * n
        nf = struct.unpack('>H', data[pos:pos + 2])[0]
        pos += 2
        self.fields = []
        for _ in range(nf):
            acc = struct.unpack('>H', data[pos:pos + 2])[0]
            pos += 2
            ni = struct.unpack('>H', data[pos:pos + 2])[0]
            pos += 2
            di = struct.unpack('>H', data[pos:pos + 2])[0]
            pos += 2
            m = struct.unpack('>H', data[pos:pos + 2])[0]
            pos += 2
            const = None
            for __ in range(m):
                an = struct.unpack('>H', data[pos:pos + 2])[0]
                pos += 2
                ln = struct.unpack('>I', data[pos:pos + 4])[0]
                pos += 4
                if jdis.resolve(self.cp, an) == 'ConstantValue':
                    ci = struct.unpack('>H', data[pos:pos + 2])[0]
                    const = self.cp[ci]
                pos += ln
            self.fields.append((acc, jdis.resolve(self.cp, ni),
                                jdis.resolve(self.cp, di), const))
        nm = struct.unpack('>H', data[pos:pos + 2])[0]
        pos += 2
        self.methods = []
        for _ in range(nm):
            acc = struct.unpack('>H', data[pos:pos + 2])[0]
            pos += 2
            ni = struct.unpack('>H', data[pos:pos + 2])[0]
            pos += 2
            di = struct.unpack('>H', data[pos:pos + 2])[0]
            pos += 2
            m = struct.unpack('>H', data[pos:pos + 2])[0]
            pos += 2
            code = None
            exc = []
            for __ in range(m):
                an = struct.unpack('>H', data[pos:pos + 2])[0]
                pos += 2
                ln = struct.unpack('>I', data[pos:pos + 4])[0]
                pos += 4
                blob = data[pos:pos + ln]
                if jdis.resolve(self.cp, an) == 'Code':
                    max_stack = struct.unpack('>H', blob[0:2])[0]
                    max_loc = struct.unpack('>H', blob[2:4])[0]
                    clen = struct.unpack('>I', blob[4:8])[0]
                    cbytes = blob[8:8 + clen]
                    p = 8 + clen
                    ne = struct.unpack('>H', blob[p:p + 2])[0]
                    p += 2
                    for __ in range(ne):
                        s, e, h, c = struct.unpack('>HHHH', blob[p:p + 8])
                        p += 8
                        exc.append((s, e, h, c))
                    code = (max_stack, max_loc, cbytes, exc)
                pos += ln
            self.methods.append((acc, jdis.resolve(self.cp, ni),
                                 jdis.resolve(self.cp, di), code))

    def strings(self):
        out = []
        for e in self.cp:
            if e and e[0] == 'Utf8' and len(e[1]) > 2 and any(
                    ch.isalpha() for ch in e[1]):
                if '(' in e[1] or e[1].startswith('L') and e[1].endswith(';'):
                    continue
                out.append(e[1])
        return out


# ---------------------------------------------------------------------------
# method body emitter
# ---------------------------------------------------------------------------

_CMP = {'ifeq': '==', 'ifne': '!=', 'iflt': '<', 'ifge': '>=',
        'ifgt': '>', 'ifle': '<=',
        'if_acmpeq': '==', 'if_acmpne': '!=',
        'if_icmpeq': '==', 'if_icmpne': '!=', 'if_icmplt': '<',
        'if_icmpge': '>=', 'if_icmpgt': '>', 'if_icmple': '<='}

_ARITH = {'iadd': '+', 'ladd': '+', 'fadd': '+', 'dadd': '+',
          'isub': '-', 'lsub': '-', 'fsub': '-', 'dsub': '-',
          'imul': '*', 'lmul': '*', 'fmul': '*', 'dmul': '*',
          'idiv': '/', 'ldiv': '/', 'fdiv': '/', 'ddiv': '/',
          'irem': '%', 'lrem': '%', 'frem': '%', 'drem': '%',
          'ishl': '<<', 'lshl': '<<', 'ishr': '>>', 'lshr': '>>',
          'iushr': '>>>', 'lushr': '>>>',
          'iand': '&', 'land': '&', 'ior': '|', 'lor': '|',
          'ixor': '^', 'lxor': '^'}

_CONV = {'i2l': 'long', 'i2f': 'float', 'i2d': 'double',
         'l2i': 'int', 'l2f': 'float', 'l2d': 'double',
         'f2i': 'int', 'f2l': 'long', 'f2d': 'double',
         'd2i': 'int', 'd2l': 'long', 'd2f': 'float',
         'i2b': 'byte', 'i2c': 'char', 'i2s': 'short',
         'l2i_dup': 'int'}


def overload_map(klass, names):
    """(name, params, ret) -> printed name, splitting return-only clashes."""
    groups = {}
    for _, mname, mdesc, _ in klass.methods:
        if mname in ('<init>', '<clinit>'):
            continue
        try:
            params, ret = split_desc(mdesc)
        except Exception:
            continue
        groups.setdefault((mname, tuple(params)), {})[ret] = 1
    omap = {}
    for (mname, params), rets in groups.items():
        if len(rets) > 1:
            for ret in rets:
                tag = java_type(ret, names).replace('.', '_')
                tag = tag.replace('[', 'arr').replace(']', '')
                tag = ''.join(ch if ch.isalnum() else '_' for ch in tag)
                omap[(mname, tuple(params), ret)] = '%s__%s' % (mname, tag)
    return omap


def _esc(s):
    return (s.replace('\\', '\\\\').replace('"', '\\"')
             .replace('\n', '\\n').replace('\r', '\\r').replace('\t', '\\t'))


class Emitter:
    """Symbolic stack machine turning one method into Java statements."""

    def __init__(self, klass, acc, name, desc, code, names, fmap=None):
        self.fmap = fmap or {}
        self._own_desc = desc
        self.klass = klass
        self._init_rest(klass, acc, name, desc, code, names)
        self.omap = overload_map(klass, names)

    def oname(self, nm, ds):
        try:
            params, ret = split_desc(ds)
        except Exception:
            return nm
        return self.omap.get((nm, tuple(params), ret), nm)

    def _init_rest(self, klass, acc, name, desc, code, names):
        self.names = names
        self.mname = name
        self.static = bool(acc & 0x0008)
        self.params, self.ret = split_desc(desc)
        self.max_stack, self.max_loc, self.bc, self.exc = code
        self.ins = decode(self.bc)
        self.targets = set()
        for _, op, arg, _ in self.ins:
            if op in _CMP or op in ('goto', 'jsr', 'ifnull', 'ifnonnull',
                                    'if_acmpeq', 'if_acmpne'):
                self.targets.add(arg)
            elif op in ('tableswitch', 'lookupswitch'):
                self.targets.add(arg[0])
                for _, t in arg[1]:
                    self.targets.add(t)
            elif op == 'ret':
                pass
        self.lines = []
        self.stack = []
        self.local_type = {}
        self.local_decl = set()
        self.new_ids = {}
        self.new_seq = 0
        self.max_depth = 0
        self.problems = []
        self._init_locals()

    # -- names ---------------------------------------------------------
    def local(self, idx, want=None):
        if idx == 0 and not self.static:
            return 'this'
        if idx not in self.local_type and want:
            self.local_type[idx] = want
        return 'v%d' % idx if (idx or self.static) else 'this'

    def _bool(self, typ, expr):
        if typ == 'boolean' and expr in ('0', '1'):
            return 'false' if expr == '0' else 'true'
        return expr

    def declare(self, idx, typ, expr):
        nm = self.local(idx, typ)
        if nm == 'this':
            self.problems.append('store to this at pc')
            self.lines.append('this = %s; /* INVALID: store to this */' % expr)
            return
        if idx not in self.local_decl:
            self.local_decl.add(idx)
            self.lines.append('%s %s = %s;' % (typ, nm, self._bool(typ, expr)))
            return
        if expr == nm:
            return
        if True:
            if self.local_type.get(idx) != typ:
                expr = '((%s) %s)' % (self.local_type[idx], expr)
            self.lines.append('%s = %s;' % (nm, expr))

    def member(self, ref):
        cls, name, desc = self._ref(ref)
        name = self.fmap.get((name, desc), name)
        return '%s.%s' % (cls, name), desc

    def static_member(self, ref):
        """Static field via its defining class (handles inheritance)."""
        full = jdis.resolve(self.klass.cp, ref)
        cls_obf, rest = full.split('.', 1)
        cls_obf = cls_obf.replace('/', '.')
        nm = rest.split(':')[0]
        ds = rest.split(':')[1] if ':' in rest else ''
        try:
            owner = self.defining_class(cls_obf, nm, ds)
        except Exception:
            owner = cls_obf
        try:
            oname = field_map(cached_klass(owner)).get((nm, ds), nm)
        except Exception:
            oname = nm
        return '%s.%s' % (readable_class(owner, self.names), oname), ds

    _field_cache = {}

    def defining_class(self, obf, name, desc):
        """Nearest class up `obf`'s hierarchy declaring field (name, desc)."""
        key = (obf, name, desc)
        if key in Emitter._field_cache:
            return Emitter._field_cache[key]
        seen = set()
        cur = obf
        found = obf
        while cur and cur not in seen:
            seen.add(cur)
            try:
                fields = [(n, d) for _, n, d, _ in cached_klass(cur).fields]
            except Exception:
                break
            if (name, desc) in fields:
                found = cur
                break
            cur = _super_of(cur)
        Emitter._field_cache[key] = found
        return found

    def _ref(self, idx):
        e = self.klass.cp[idx]
        tag = e[0]
        if tag in (9, 10, 11):
            cls = jdis.resolve(self.klass.cp, e[1])
            nm, ds = jdis.resolve(self.klass.cp, e[2]).split(':', 1) \
                if ':' in jdis.resolve(self.klass.cp, e[2]) \
                else (jdis.resolve(self.klass.cp, e[2]), '')
            if tag == 9:
                full = jdis.resolve(self.klass.cp, idx)
                cls, rest = full.split('.', 1)
                nm, ds = rest.split(':', 1) if ':' in rest else (rest, '')
            return (readable_class(cls, self.names)
                    if '/' in cls or '.' not in cls and len(cls) <= 2
                    else cls.split('/')[-1].split('.')[-1], nm, ds)
        if tag == 12:
            return '', jdis.resolve(self.klass.cp, e[1]), \
                jdis.resolve(self.klass.cp, e[2])
        return '', jdis.resolve(self.klass.cp, idx), ''

    def meth_name(self, cls_read, nm):
        if nm == '<init>':
            return None
        return nm

    def const(self, idx):
        e = self.klass.cp[idx]
        if e[0] == 'Utf8':
            return 'String', '"%s"' % _esc(e[1])
        tag = e[0]
        if tag == 8:
            raw = jdis.resolve(self.klass.cp, e[1])
            return 'String', '"%s"' % _esc(raw)
        if tag == 3:
            return 'int', str(e[1])
        if tag == 4:
            v = e[1]
            if math.isnan(v):
                return 'float', 'Float.NaN'
            if math.isinf(v):
                return 'float', '(Float.POSITIVE_INFINITY)' if v > 0 else \
                    '(-Float.POSITIVE_INFINITY)'
            return 'float', repr(float(v)) + 'f'
        if tag == 5:
            return 'long', str(e[1]) + 'L'
        if tag == 6:
            v = e[1]
            if math.isnan(v):
                return 'double', 'Double.NaN'
            if math.isinf(v):
                return 'double', '(Double.POSITIVE_INFINITY)' if v > 0 else \
                    '(-Double.POSITIVE_INFINITY)'
            return 'double', repr(float(v))
        if tag == 7:
            cn = jdis.resolve(self.klass.cp, e[1]).replace('/', '.')
            if cn.startswith('java.lang.'):
                return 'Class', cn[10:] + '.class'
            return 'Class', readable_class(cn, self.names) + '.class'
        self.problems.append('strange ldc tag %s' % (tag,))
        return 'Object', '/* ldc tag %s #%d */ null' % (tag, idx)

    # -- init ----------------------------------------------------------
    def _init_locals(self):
        idx = 0
        if not self.static:
            self.local_type[0] = readable_class(self.klass.obf, self.names)
            idx = 1
        for p in self.params:
            t = java_type(p, self.names)
            if p in ('J', 'D'):
                self.local_type[idx] = t
                idx += 2
            else:
                self.local_type[idx] = t
                idx += 1

    def sig(self):
        me = readable_class(self.klass.obf, self.names)
        if self.mname == '<init>':
            head = 'public %s(' % me
        elif self.mname == '<clinit>':
            return None
        else:
            head = 'public %s%s %s(' % (
                'static ' if self.static else '',
                java_type(self.ret, self.names),
                self.oname(self.mname, '(%s)%s' % (''.join(self.params),
                                                   self.ret)))
        parts = []
        idx = 0 if self.static else 1
        for p in self.params:
            t = java_type(p, self.names)
            parts.append('%s v%d' % (t, idx))
            idx += 2 if p in ('J', 'D') else 1
        self.local_decl.update(range(0 if self.static else 1, idx))
        return head + ', '.join(parts) + ')'

    # -- run -----------------------------------------------------------
    def push(self, typ, expr):
        self.stack.append((typ, expr))
        self.max_depth = max(self.max_depth, len(self.stack))

    def pop(self):
        if not self.stack:
            self.problems.append('stack underflow')
            return 'Object', '/* UNDERFLOW */ null'
        return self.stack.pop()

    def run(self):
        self._blocks()
        self.entry = {self.blocks[0]: []}
        self.temps = 0
        # exception handlers enter with the exception on the stack
        for s, e, h, c in self.exc:
            cn = 'java.lang.Throwable' if c == 0 else jdis.resolve(
                self.klass.cp, c).replace('/', '.')
            if cn.startswith('java.') or cn.startswith('javax.'):
                ct = cn
            else:
                ct = readable_class(cn, self.names)
            if h not in self.entry:
                self.entry[h] = [(ct, 'ex_%d' % h)]
        self.processed = set()
        self.bufs = {}
        self.bufpos = {}
        self.offered_by = {}
        for b in self.blocks:
            self._run_block(b)
        return self.lines

    def _blocks(self):
        starts = {0}
        for pc, op, arg, ln in self.ins:
            if op in _CMP or op in ('goto', 'jsr', 'ifnull', 'ifnonnull',
                                    'if_acmpeq', 'if_acmpne'):
                starts.add(arg)
                starts.add(pc + ln)
            elif op in ('tableswitch', 'lookupswitch'):
                starts.add(arg[0])
                for _, t in arg[1]:
                    starts.add(t)
                starts.add(pc + ln)
            elif op in ('ireturn', 'lreturn', 'freturn', 'dreturn',
                        'areturn', 'return', 'athrow', 'ret'):
                starts.add(pc + ln)
        for s, e, h, c in self.exc:
            starts.add(h)
        maxpc = self.ins[-1][0] + self.ins[-1][3] if self.ins else 0
        starts = sorted(p for p in starts if p <= maxpc)
        self.blocks = starts
        self.bmap = {pc: (op, arg, ln) for pc, op, arg, ln in self.ins}

    def _offer(self, target, stack, pred):
        if target not in self.entry:
            self.entry[target] = list(stack)
            self.offered_by[target] = pred
            return
        cur = self.entry[target]
        if target in self.processed:
            for k in range(min(len(cur), len(stack))):
                if cur[k][1] != stack[k][1] or len(cur) != len(stack):
                    self.problems.append(
                        'late join into emitted L%d differs' % target)
                    break
            return
        if len(cur) != len(stack):
            self.problems.append(
                'join L%d depth %d vs %d' % (target, len(cur), len(stack)))
            return
        for k in range(len(cur)):
            if cur[k][1] != stack[k][1]:
                self.temps += 1
                tn = 'jt%d' % self.temps
                first_pred = self.offered_by.get(target)
                first_expr = cur[k][1]
                if first_pred is not None and first_pred in self.bufs:
                    buf = self.bufs[first_pred]
                    line = ('%s = %s; /* join L%d from L%d */'
                            % (tn, first_expr, target, first_pred))
                    at = len(buf)
                    if buf and (buf[-1].startswith('goto L')
                                or buf[-1].startswith('if ')
                                or buf[-1].startswith('return')
                                or buf[-1].startswith('throw')):
                        at -= 1
                    buf.insert(at, line)
                    if first_pred in self.bufpos:
                        start, end = self.bufpos[first_pred]
                        self.lines.insert(start + at, line)
                        for key in self.bufpos:
                            s2, e2 = self.bufpos[key]
                            if s2 >= start + at and key != first_pred:
                                self.bufpos[key] = (s2 + 1, e2 + 1)
                        self.bufpos[first_pred] = (start, end + 1)
                self.lines.append(
                    '%s = %s; /* join L%d from L%d */'
                    % (tn, stack[k][1], target, pred))
                cur[k] = (cur[k][0], tn)
                self.offered_by[target] = pred

    def _run_block(self, b):
        if b in self.processed:
            return
        if not any(pc >= b and (not [x for x in self.blocks if x > b]
                                or pc < [x for x in self.blocks if x > b][0])
                   for pc in self.bmap):
            return
        self.processed.add(b)
        self.stack = list(self.entry.get(b, []))
        if b and b not in self.entry:
            self.problems.append('block L%d with no entry stack' % b)
            self.lines.append('/* NOTE: block L%d reached with empty stack '
                              '(subroutine or handler?) */' % b)
        buf = []
        save = self.lines
        self.lines = buf
        pcs = [pc for pc in sorted(self.bmap) if pc >= b]
        nxt = [x for x in self.blocks if x > b]
        end = nxt[0] if nxt else None
        last = None
        for pc in pcs:
            if end is not None and pc >= end:
                break
            last = pc
            if pc in self.targets and pc != b:
                self.lines.append('L%d:' % pc)
                break
            if pc in self.targets:
                self.lines.append('L%d:' % pc)
            op, arg, _ = self.bmap[pc]
            self.step(pc, op, arg)
            if op in ('goto', 'jsr', 'ret', 'ireturn', 'lreturn',
                      'freturn', 'dreturn', 'areturn', 'return', 'athrow') \
                    or op in ('tableswitch', 'lookupswitch') \
                    or op in _CMP or op in ('ifnull', 'ifnonnull',
                                            'if_acmpeq', 'if_acmpne'):
                break
        self.bufpos[b] = (len(save), len(save) + len(buf))
        save.extend(buf)
        self.bufs[b] = buf
        self.lines = save
        if last is None:
            return
        op, arg, ln = self.bmap[last]
        exit_stack = list(self.stack)
        if op == 'goto':
            self._offer(arg, exit_stack, b)
        elif op in _CMP or op in ('ifnull', 'ifnonnull', 'if_acmpeq',
                                  'if_acmpne'):
            self._offer(arg, exit_stack, b)
            if end is not None:
                self._offer(end, exit_stack, b)
        elif op in ('tableswitch', 'lookupswitch'):
            self._offer(arg[0], exit_stack, last)
            for _, t in arg[1]:
                self._offer(t, exit_stack, last)
        elif op == 'jsr':
            self._offer(arg, exit_stack, b)
            if end is not None:
                self._offer(end, exit_stack, b)
        elif op in ('ireturn', 'lreturn', 'freturn', 'dreturn', 'areturn',
                    'return', 'athrow', 'ret'):
            pass
        else:
            if end is not None:
                self._offer(end, exit_stack, b)

    def run_old(self):
        for pc, op, arg, _ in self.ins:
            if pc in self.targets:
                self.lines.append('L%d:' % pc)
            self.step(pc, op, arg)
        return self.lines

    def _load_idx(self, op, arg):
        if '_' in op:
            try:
                return int(op.rsplit('_', 1)[1])
            except ValueError:
                pass
        return arg

    # -- one instruction, part 1: constants, locals, arithmetic --------
    def step(self, pc, op, arg):
        L = self.lines
        P = self.push
        if op.startswith('iconst_'):
            P('int', op[7:])
            return
        if op.startswith('fconst_'):
            P('float', op[7:] + '.0f')
            return
        if op.startswith('lconst_'):
            P('long', op[7:] + 'L')
            return
        if op.startswith('dconst_'):
            P('double', op[7:] + '.0')
            return
        if op in ('bipush', 'sipush'):
            P('int', str(arg))
            return
        if op in ('ldc', 'ldc_w', 'ldc2_w'):
            t, e = self.const(arg)
            P(t, e)
            return
        if op.startswith('iload') or op.startswith('lload') or \
                op.startswith('fload') or op.startswith('dload') or \
                op.startswith('aload'):
            idx = self._load_idx(op, arg)
            base = {'i': 'int', 'l': 'long', 'f': 'float', 'd': 'double',
                    'a': 'Object'}[op[0]]
            P(self.local_type.get(idx, base), self.local(idx, base))
            return
        if op.startswith('istore') or op.startswith('lstore') or \
                op.startswith('fstore') or op.startswith('dstore') or \
                op.startswith('astore'):
            idx = self._load_idx(op, arg)
            base = {'i': 'int', 'l': 'long', 'f': 'float', 'd': 'double',
                    'a': None}[op[0]]
            t, e = self.pop()
            self.declare(idx, base or t, e)
            return
        if op == 'iinc':
            idx, c = arg
            if c > 127:
                c -= 256
            nm = self.local(idx, 'int')
            if idx not in self.local_decl:
                self.local_decl.add(idx)
                L.append('int %s = 0; /* iinc before set */' % nm)
            L.append('%s += %d;' % (nm, c))
            return
        if op in _ARITH:
            t2, b = self.pop()
            t1, a = self.pop()
            if t1 in ('long', 'float', 'double'):
                t = t1
            elif t2 in ('long', 'float', 'double'):
                t = t2
            else:
                t = 'int'
            P(t, '(%s %s %s)' % (a, _ARITH[op], b))
            return
        if op in ('ineg', 'lneg', 'fneg', 'dneg'):
            t, a = self.pop()
            P(t, '(-%s)' % a)
            return
        if op in _CONV:
            t, a = self.pop()
            P(_CONV[op], '((%s) %s)' % (_CONV[op], a))
            return
        if op in ('lcmp', 'fcmpg', 'fcmpl', 'dcmpg', 'dcmpl'):
            _, b = self.pop()
            _, a = self.pop()
            P('int', '%s(%s, %s)' % (op, a, b))
            return
        if op in _CMP:
            if op.startswith('if_icmp') or op in ('if_acmpeq', 'if_acmpne'):
                _, b = self.pop()
                _, a = self.pop()
                L.append('if (%s %s %s) goto L%d;' % (a, _CMP[op], b, arg))
            else:
                _, a = self.pop()
                L.append('if (%s %s 0) goto L%d;' % (a, _CMP[op], arg))
            return
        if op in ('ifnull', 'ifnonnull'):
            _, a = self.pop()
            L.append('if (%s %s null) goto L%d;'
                     % (a, '==' if op == 'ifnull' else '!=', arg))
            return
        if op == 'goto':
            L.append('goto L%d;' % arg)
            return
        if op == 'jsr':
            L.append('jsr L%d; /* subroutine: inline when reviewing */' % arg)
            return
        if op == 'ret':
            L.append('ret v%d; /* subroutine return */' % arg)
            return
        if op in ('tableswitch', 'lookupswitch'):
            _, a = self.pop()
            default, cases = arg
            L.append('switch (%s) {' % a)
            for k, t in cases:
                L.append('case %d: goto L%d;' % (k, t))
            L.append('default: goto L%d;' % default)
            L.append('}')
            return
        if op in ('ireturn', 'lreturn', 'freturn', 'dreturn', 'areturn'):
            _, a = self.pop()
            L.append('return %s;' % a)
            return
        if op == 'return':
            L.append('return;')
            return
        if op == 'athrow':
            _, a = self.pop()
            L.append('throw %s;' % a)
            return
        if op == 'aconst_null':
            P('Object', 'null')
            return
        if op == 'pop':
            t, e = self.pop()
            if len(e) > 4 and not e.startswith('jt'):
                L.append('/* pop: %s; */' % e)
            return
        if op == 'pop2':
            t, e = self.pop()
            if t not in ('long', 'double'):
                self.pop()
            if len(e) > 4 and not e.startswith('jt'):
                L.append('/* pop: %s; */' % e)
            return
        if op == 'dup':
            t, a = self.pop()
            self.push(t, a)
            self.push(t, a)
            return
        if op == 'dup_x1':
            t1, a = self.pop()
            t2, b = self.pop()
            self.push(t1, a)
            self.push(t2, b)
            self.push(t1, a)
            return
        if op == 'dup_x2':
            t1, a = self.pop()
            t2, b = self.pop()
            if t1 in ('long', 'double'):
                self.problems.append('dup_x2 on 2-slot value at %d' % pc)
                self.push(t1, a)
                self.push(t2, b)
                self.push(t1, a)
                return
            if t2 in ('long', 'double'):
                self.push(t1, a)
                self.push(t2, b)
                self.push(t1, a)
                return
            t3, c = self.pop()
            self.push(t1, a)
            self.push(t3, c)
            self.push(t2, b)
            self.push(t1, a)
            return
        if op == 'dup2':
            t1, a = self.pop()
            if t1 in ('long', 'double'):
                self.push(t1, a)
                self.push(t1, a)
                return
            t2, b = self.pop()
            self.push(t2, b)
            self.push(t1, a)
            self.push(t2, b)
            self.push(t1, a)
            return
        if op == 'dup2_x1':
            t1, a = self.pop()
            t2, b = self.pop()
            if t1 in ('long', 'double'):
                self.problems.append('dup2_x1 on 2-slot value at %d' % pc)
                self.push(t1, a)
                self.push(t2, b)
                self.push(t1, a)
                return
            if t2 in ('long', 'double'):
                self.push(t1, a)
                self.push(t2, b)
                self.push(t1, a)
                return
            t3, c = self.pop()
            self.push(t2, b)
            self.push(t1, a)
            self.push(t3, c)
            self.push(t2, b)
            self.push(t1, a)
            return
        if op == 'dup2_x2':
            t1, a = self.pop()
            t2, b = self.pop()
            if t1 in ('long', 'double') and t2 in ('long', 'double'):
                self.push(t1, a)
                self.push(t2, b)
                self.push(t1, a)
                return
            if t1 in ('long', 'double'):
                t3, c = self.pop()
                self.push(t1, a)
                self.push(t3, c)
                self.push(t2, b)
                self.push(t1, a)
                return
            if t2 in ('long', 'double'):
                self.push(t1, a)
                self.push(t2, b)
                self.push(t1, a)
                return
            t3, c = self.pop()
            t4, d = self.pop()
            for t, v in [(t2, b), (t1, a), (t4, d), (t3, c),
                         (t2, b), (t1, a)]:
                self.push(t, v)
            return
        if op == 'swap':
            t1, a = self.pop()
            t2, b = self.pop()
            self.push(t1, a)
            self.push(t2, b)
            return
        self.step2(pc, op, arg)

    # -- one instruction, part 2: arrays, objects, calls ---------------
    def step2(self, pc, op, arg):
        L = self.lines
        P = self.push
        if op in ('iaload', 'laload', 'faload', 'daload', 'aaload',
                  'baload', 'caload', 'saload'):
            _, i = self.pop()
            _, a = self.pop()
            t = {'iaload': 'int', 'laload': 'long', 'faload': 'float',
                 'daload': 'double', 'aaload': 'Object', 'baload': 'byte',
                 'caload': 'char', 'saload': 'short'}[op]
            P(t, '%s[%s]' % (a, i))
            return
        if op in ('iastore', 'lastore', 'fastore', 'dastore', 'aastore',
                  'bastore', 'castore', 'sastore'):
            _, v = self.pop()
            _, i = self.pop()
            _, a = self.pop()
            L.append('%s[%s] = %s;' % (a, i, v))
            return
        if op == 'arraylength':
            _, a = self.pop()
            P('int', '%s.length' % a)
            return
        if op == 'newarray':
            _, n = self.pop()
            base = _NEWARRAY.get(arg, 'ATYPE%d' % arg)
            self.new_seq += 1
            self.new_ids[self.new_seq] = [base, None]
            P(base, 'new %s[%s]#%d' % (base, n, self.new_seq))
            return
        if op == 'anewarray':
            _, n = self.pop()
            cls = readable_class(jdis.resolve(
                self.klass.cp, self.klass.cp[arg][1]).replace('/', '.'),
                self.names)
            self.new_seq += 1
            self.new_ids[self.new_seq] = [cls, None]
            P(cls + '[]', 'new %s[%s]#%d' % (cls, n, self.new_seq))
            return
        if op == 'multianewarray':
            idx, dims = arg
            desc = jdis.resolve(self.klass.cp, idx)
            sizes = [self.pop()[1] for _ in range(dims)]
            sizes.reverse()
            base = java_type(desc, self.names)
            while base.endswith('[]'):
                base = base[:-2]
            self.new_seq += 1
            self.new_ids[self.new_seq] = [base, None]
            P(java_type(desc, self.names),
              'new %s%s#%d' % (base, ''.join('[%s]' % s for s in sizes),
                               self.new_seq))
            return
        if op == 'new':
            cn = jdis.resolve(self.klass.cp,
                              self.klass.cp[arg][1]).replace('/', '.')
            cls = readable_class(cn, self.names)
            self.new_seq += 1
            expr = 'new %s#%d' % (cls, self.new_seq)
            self.new_ids[self.new_seq] = [cls, None]
            P(cls, expr)
            return
        if op in ('getstatic', 'getfield'):
            full, _ = self.static_member(arg) if op == 'getstatic' \
                else self.member(arg)
            if op == 'getstatic':
                P(self._field_type(arg), full)
            else:
                _, recv = self.pop()
                P(self._field_type(arg),
                  '%s.%s' % (recv, full.split('.', 1)[1]))
            return
        if op in ('putstatic', 'putfield'):
            _, v = self.pop()
            full, fdesc = self.static_member(arg) if op == 'putstatic' \
                else self.member(arg)
            ftype = java_type(fdesc, self.names) if fdesc else 'Object'
            if op == 'putstatic':
                L.append('%s = %s;' % (full, self._bool(ftype, v)))
            else:
                _, recv = self.pop()
                L.append('%s.%s = %s;'
                         % (recv, full.split('.', 1)[1],
                            self._bool(ftype, v)))
            return
        if op in ('invokevirtual', 'invokespecial', 'invokestatic',
                  'invokeinterface'):
            self._invoke(op, arg, L, P)
            return
        if op == 'checkcast':
            cn = jdis.resolve(self.klass.cp,
                              self.klass.cp[arg][1]).replace('/', '.')
            cls = readable_class(cn, self.names)
            _, a = self.pop()
            P(cls, '((%s) %s)' % (cls, a))
            return
        if op == 'instanceof':
            cn = jdis.resolve(self.klass.cp,
                              self.klass.cp[arg][1]).replace('/', '.')
            cls = readable_class(cn, self.names)
            _, a = self.pop()
            P('boolean', '(%s instanceof %s)' % (a, cls))
            return
        if op == 'monitorenter':
            _, a = self.pop()
            L.append('monitorenter(%s); /* synchronized block */' % a)
            return
        if op == 'monitorexit':
            _, a = self.pop()
            L.append('monitorexit(%s);' % a)
            return
        if op == 'nop':
            return
        self.problems.append('unhandled %s at %d' % (op, pc))
        L.append('/* UNHANDLED %s %s at pc %d */' % (op, arg, pc))

    def _field_type(self, idx):
        full = jdis.resolve(self.klass.cp, idx)
        try:
            _, ds = full.split(':', 1)
            return java_type(ds, self.names)
        except ValueError:
            return 'Object'

    def _invoke(self, op, arg, L, P):
        if isinstance(arg, tuple):
            arg = arg[0]
        full = jdis.resolve(self.klass.cp, arg)
        cls, rest = full.split('.', 1)
        nm = rest.split(':')[0]
        ds = rest.split(':')[1] if ':' in rest else ''
        cls_r = readable_class(cls.replace('/', '.'), self.names)
        params, ret = split_desc(ds) if ds.startswith('(') else ([], 'V')
        nm = self.oname(nm, ds) if ds.startswith('(') else nm
        nargs = len(params)
        args = [self.pop()[1] for _ in range(nargs)]
        args.reverse()
        for i, p in enumerate(params):
            if i < len(args) and p == 'Z' and args[i] in ('0', '1'):
                args[i] = 'false' if args[i] == '0' else 'true'
        recv = None
        if op != 'invokestatic':
            _, recv = self.pop()
        call = '%s(%s)' % (nm, ', '.join(args))
        if nm == '<init>' and recv == 'this':
            others = [d for _, m, d, _ in self.klass.methods
                      if m == '<init>' and d != self._own_desc]
            if ds in others:
                L.append('this(%s);' % ', '.join(args))
            else:
                L.append('super(%s);' % ', '.join(args))
            return
        if nm == '<init>':
            import re as _re
            mm = _re.match(r'^new (\S+)#(\d+)$', recv or '')
            if mm:
                nid = int(mm.group(2))
                resolved = 'new %s(%s)' % (mm.group(1), ', '.join(args))
                self.new_ids[nid][1] = resolved
                for k in range(len(self.stack)):
                    if self.stack[k][1] == recv:
                        self.stack[k] = (self.stack[k][0], resolved)
                L.append('%s; /* constructor */' % resolved)
            else:
                L.append('%s.<init>(%s);' % (recv, ', '.join(args)))
            return
        if op == 'invokestatic':
            expr = '%s.%s' % (cls_r, call)
        else:
            expr = '%s.%s' % (recv, call)
        if ret == 'V':
            L.append(expr + ';')
        else:
            P(java_type(ret, self.names), expr)


# ---------------------------------------------------------------------------
# whole-class emitter
# ---------------------------------------------------------------------------

ACC_S = {0x0001: 'public', 0x0010: 'final', 0x0200: 'interface',
         0x0400: 'abstract', 0x0020: 'synchronized', 0x0100: 'native',
         0x0080: 'transient', 0x0040: 'volatile', 0x0008: 'static'}


def _plain_sig(klass, macc, mname, mdesc, names):
    me = readable_class(klass.obf, names)
    static = bool(macc & 0x0008)
    if mname == '<init>':
        head = 'public %s(' % me
    elif mname == '<clinit>':
        return None
    else:
        params, ret = split_desc(mdesc)
        head = 'public %s%s %s(' % (
            'static ' if static else '', java_type(ret, names), mname)
    params, _ = split_desc(mdesc)
    parts = []
    idx = 0 if static else 1
    for p in params:
        parts.append('%s v%d' % (java_type(p, names), idx))
        idx += 2 if p in ('J', 'D') else 1
    return head + ', '.join(parts) + ')'


def acc_str(acc, kind):
    parts = []
    if acc & 0x0001:
        parts.append('public')
    if acc & 0x0002:
        parts.append('private')
    if acc & 0x0004:
        parts.append('protected')
    if acc & 0x0008:
        parts.append('static')
    if acc & 0x0010:
        parts.append('final')
    if kind == 'method':
        if acc & 0x0020:
            parts.append('synchronized')
        if acc & 0x0100:
            parts.append('native')
        if acc & 0x0080:
            parts.append('strictfp')
    else:
        if acc & 0x0040:
            parts.append('volatile')
        if acc & 0x0080:
            parts.append('transient')
    return ' '.join(parts)


def const_str(const, cp, names):
    if const is None:
        return None
    tag = const[0]
    if tag in (3,):
        return str(const[1])
    if tag in (4,):
        v = const[1]
        if math.isnan(v):
            return 'Float.NaN'
        if math.isinf(v):
            return '(Float.POSITIVE_INFINITY)' if v > 0 else \
                '(-Float.POSITIVE_INFINITY)'
        return repr(float(v)) + 'f'
    if tag in (5,):
        return str(const[1]) + 'L'
    if tag in (6,):
        v = const[1]
        if math.isnan(v):
            return 'Double.NaN'
        if math.isinf(v):
            return '(Double.POSITIVE_INFINITY)' if v > 0 else \
                '(-Double.POSITIVE_INFINITY)'
        return repr(float(v))
    if tag == 8:
        return '"%s"' % _esc(jdis.resolve(cp, const[1]))
    return '/* ConstantValue tag %s */ null' % tag


_KLASS_CACHE = {}


def cached_klass(obf):
    if obf not in _KLASS_CACHE:
        _KLASS_CACHE[obf] = Klass(obf)
    return _KLASS_CACHE[obf]


def _super_of(obf):
    """Superclass obfuscated name, or None for java/ roots."""
    try:
        data, pos0, cp = jdis.load(os.path.join(_XDIR, obf + '.class'))
    except Exception:
        return None
    v = struct.unpack('>H', data[pos0 + 4:pos0 + 6])[0]
    r = jdis.resolve(cp, v)
    if r.startswith('#') or '/' in r or r == 'java/lang/Object':
        return None
    return r


_FMAP_CACHE = {}


def field_map(klass):
    """Disambiguated field names for one class."""
    obf = klass.obf
    if obf in _FMAP_CACHE:
        return _FMAP_CACHE[obf]
    seen = {}
    fmap = {}
    for _, fname, fdesc, _ in klass.fields:
        key = (fname, fdesc)
        if fname in seen:
            uniq = '%s_%s' % (fname, fdesc.strip('L;').split('/')[-1][:6]
                              .replace('[', 'arr').replace('.', '_'))
            n = 2
            while uniq in seen.values():
                uniq = '%s_%d' % (uniq, n)
                n += 1
            fmap[key] = uniq
            seen[fname] = uniq
        else:
            seen[fname] = fname
            fmap[key] = fname
    _FMAP_CACHE[obf] = fmap
    return fmap


def role_note(klass):
    """One-line role guess from strings, superclass and API usage."""
    strs = klass.strings()
    refs = set()
    for e in klass.cp:
        if e and e[0] in (9, 10, 11):
            full = jdis.resolve(klass.cp, klass.cp.index(e))
            refs.add(full.split('.')[0])
    apis = sorted(set(r.split('/')[0] + '/' + r.split('/')[1]
                      if '/' in r else r for r in refs
                      if '/' in r and not r.startswith('java/lang')))
    keys = [s for s in strs if len(s) < 60]
    return keys[:12], apis[:12]


def decompile_class(obf, names):
    klass = Klass(obf)
    me = readable_class(obf, names)
    keys, apis = role_note(klass)
    out = []
    out.append('// Generated by tools/jdeob.py from x/%s.class.  Do not hand' % obf)
    out.append('// edit: fix the emitter and re-run.  Original name `%s`.' % obf)
    if keys:
        out.append('// String constants: %s' % ' | '.join(keys))
    if apis:
        out.append('// API references: %s' % ' '.join(apis))
    # superclass and interfaces: read from the class header
    data, pos0, cp = jdis.load(os.path.join(_XDIR, obf + '.class'))
    pos = pos0
    acc = struct.unpack('>H', data[pos:pos + 2])[0]
    pos += 2
    this = jdis.resolve(cp, struct.unpack('>H', data[pos:pos + 2])[0])
    pos += 2
    sup = jdis.resolve(cp, struct.unpack('>H', data[pos:pos + 2])[0])
    pos += 2
    nic = struct.unpack('>H', data[pos:pos + 2])[0]
    pos += 2
    ifcs = [jdis.resolve(cp, struct.unpack('>H', data[pos + 2 * k:pos + 2 * k + 2])[0])
            for k in range(nic)]

    def cname(c):
        c = c.replace('/', '.')
        if c.startswith('java.') or c.startswith('javax.'):
            return c
        return readable_class(c, names)

    is_ifc = bool(acc & 0x0200)
    kind = 'interface' if is_ifc else 'class'
    decl = 'public %s %s' % (kind, me)
    if sup != 'java/lang/Object' and not is_ifc:
        decl += ' extends %s' % cname(sup)
    if ifcs:
        decl += (' extends ' if is_ifc else ' implements ') + \
            ', '.join(cname(c) for c in ifcs)
    out.append(decl + ' {')
    fmap = field_map(klass)
    for facc, fname, fdesc, const in klass.fields:
        init = const_str(const, klass.cp, names)
        out.append('    %s %s %s%s;' % (
            acc_str(facc, 'field'), java_type(fdesc, names),
            fmap[(fname, fdesc)],
            ' = %s' % init if init else ''))
    clinit = [m for m in klass.methods if m[1] == '<clinit>']
    for _, _, _, code in clinit:
        if code:
            em = Emitter(klass, 0x0008, '<clinit>', '()V', code, names,
                         fmap)
            out.append('    static {')
            out += ['        ' + ln for ln in em.run()]
            out.append('    }')
    for macc, mname, mdesc, code in klass.methods:
        if mname in ('<init>', '<clinit>'):
            tag = 'constructor' if mname == '<init>' else 'static-init'
        else:
            tag = None
        if mname == '<clinit>':
            continue
        if code is None or (macc & 0x0100) or (macc & 0x0400):
            sig = _plain_sig(klass, macc, mname, mdesc, names)
            out.append('    %s;' % sig if sig else
                       '    /* <clinit> without code */')
            continue
        em = Emitter(klass, macc, mname, mdesc, code, names, fmap)
        sig = em.sig()
        out.append('    // bytecode %d bytes, max_stack %d, max_locals %d' % (
            len(code[2]), code[0], code[1]))
        out.append('    %s {' % sig)
        if code[3]:
            for s, e, h, c in code[3]:
                cn = 'finally' if c == 0 else cname(
                    jdis.resolve(klass.cp, c))
                out.append('        /* try L%d-L%d catch %s -> L%d */' %
                           (s, e, cn, h))
        body = em.run()
        out += ['        ' + ln for ln in body]
        out.append('    }')
        if em.problems:
            out.append('    /* REVIEW: %s */' % '; '.join(em.problems[:6]))
    out.append('}')
    out.append('')
    problems = sum((em.problems for em in []), [])
    return '\n'.join(out) + '\n'


def emit_all(only=None):
    names = load_names()
    # pass 1: every class gets a readable name first so references resolve
    obs = []
    for fn in sorted(os.listdir(_XDIR)):
        if fn.endswith('.class'):
            obs.append(fn[:-6])
    for ob in obs:
        readable_class(ob, names)
    if only:
        obs = [o for o in obs if o in only]
    # keep the curated hand-written race files: never overwrite them
    curated = {'am': 'M3GNode', 'at': 'MeshLoader', 'ar': 'TrackTile',
               'a': 'TrackTile', 'bc': 'MidDetail', 'bp': 'HighDetail',
               'ai': 'SceneryObject', 'bm': 'TrackCell', 'bs': 'GameTrack',
               'cl': 'RaceCar', 'bt': 'RaceController', 'bd': 'RaceStage',
               'bh': 'RaceStage', 'r': 'RaceConfig', 'bq': 'Renderer',
               'j': 'CameraRig', 'b': 'SceneryList'}
    if not only:
        # full run only: drop stale files left by renames now that
        # `curated` exists
        import glob as _g
        keep = set()
        for ob in obs:
            if ob in curated:
                keep.add('Gen' + names.get(ob, 'Obf' + ob) + '.java')
            else:
                keep.add(names[ob] + '.java')
        for path in _g.glob(os.path.join(_JDIR, 'Obf*.java')) + \
                _g.glob(os.path.join(_JDIR, 'Gen*.java')):
            if os.path.basename(path) not in keep:
                os.remove(path)
    written = []
    for ob in obs:
        text = decompile_class(ob, names)
        if ob in curated:
            fn = 'Gen' + names.get(ob, 'Obf' + ob) + '.java'
        else:
            fn = names[ob] + '.java'
        with open(os.path.join(_JDIR, fn), 'w') as fh:
            fh.write(text)
        written.append((ob, fn))
    with open(os.path.join(_JDIR, 'OBFUSCATION.md'), 'w') as fh:
        fh.write('# Obfuscated -> readable name map\n\n')
        fh.write('Generated files (`Obf*.java`, `Gen*.java`, `KORa.java`,\n')
        fh.write('`VservManager.java`) come from `tools/jdeob.py` and must\n')
        fh.write('not be hand edited. The curated race-path files\n')
        fh.write('(e.g. `GameTrack.java` for `bs`) are the reviewed layer;\n')
        fh.write('the matching `Gen*.java` file is the raw emitter output\n')
        fh.write('kept beside it for review.\n\n')
        fh.write('| original | file | role |\n')
        fh.write('|---|---|---|\n')
        for ob, fn in written:
            try:
                k = Klass(ob)
                keys, apis = role_note(k)
                role = '; '.join((keys[:4] + apis[:3]))
            except Exception as ex:
                role = 'PARSE NOTE %s' % ex
            fh.write('| `%s` | `%s` | %s |\n' % (ob, fn, role[:100]))
    return written


if __name__ == '__main__':
    emit_all(sys.argv[1:] or None)
