import struct, sys

OP = {}
def op(code, name, fmt=''):
    OP[code]=(name,fmt)

for i in range(6):
    op(0x02+i, 'iconst_m1' if i==0 else f'iconst_{i-1}')
op(0x03,'iconst_0'); op(0x04,'iconst_1'); op(0x05,'iconst_2'); op(0x06,'iconst_3'); op(0x07,'iconst_4'); op(0x08,'iconst_5')
op(0x10,'bipush','b'); op(0x11,'sipush','s')
op(0x12,'ldc','B'); op(0x13,'ldc_w','H'); op(0x14,'ldc2_w','H')
op(0x0b,'fconst_0'); op(0x0c,'fconst_1'); op(0x0d,'fconst_2')
op(0x0e,'dconst_0'); op(0x0f,'dconst_1')
for i in range(4):
    op(0x1a+i, f'iload_{i}'); op(0x22+i, f'fload_{i}'); op(0x2a+i, f'aload_{i}')
    op(0x3b+i, f'istore_{i}'); op(0x43+i, f'fstore_{i}'); op(0x4b+i, f'astore_{i}')
op(0x15,'iload','B'); op(0x17,'fload','B'); op(0x19,'aload','B')
op(0x36,'istore','B'); op(0x38,'fstore','B'); op(0x3a,'astore','B')
op(0xbc,'newarray','B'); op(0xbd,'anewarray','H'); op(0xbb,'new','H')
op(0x59,'dup'); op(0x57,'pop'); op(0x5f,'swap')
op(0xb2,'getstatic','H'); op(0xb3,'putstatic','H'); op(0xb4,'getfield','H'); op(0xb5,'putfield','H')
op(0xb6,'invokevirtual','H'); op(0xb7,'invokespecial','H'); op(0xb8,'invokestatic','H'); op(0xb9,'invokeinterface','H2')
op(0xac,'ireturn'); op(0xae,'freturn'); op(0xb0,'areturn'); op(0xb1,'return')
op(0x60,'iadd'); op(0x62,'fadd'); op(0x64,'isub'); op(0x66,'fsub'); op(0x68,'imul'); op(0x6a,'fmul'); op(0x6c,'idiv'); op(0x6e,'fdiv'); op(0x74,'ineg'); op(0x76,'fneg')
op(0x84,'iinc','BB'); op(0xa7,'goto','h')
for c,n in [(0x99,'ifeq'),(0x9a,'ifne'),(0x9b,'iflt'),(0x9c,'ifge'),(0x9d,'ifgt'),(0x9e,'ifle'),(0x9f,'if_icmpeq'),(0xa0,'if_icmpne'),(0xa1,'if_icmplt'),(0xa2,'if_icmpge'),(0xa3,'if_icmpgt'),(0xa4,'if_icmple'),(0xa5,'if_acmpeq'),(0xa6,'if_acmpne'),(0xc6,'ifnull'),(0xc7,'ifnonnull')]:
    op(c,n,'h')
op(0x2e,'iaload'); op(0x30,'faload'); op(0x32,'aaload'); op(0x33,'baload'); op(0x34,'caload')
op(0x4f,'iastore'); op(0x51,'fastore'); op(0x53,'aastore'); op(0x54,'bastore')
op(0xbe,'arraylength'); op(0xc0,'checkcast','H'); op(0xc1,'instanceof','H')

def load(path):
    data=open(path,'rb').read()
    pos=8
    cp_count=struct.unpack('>H',data[pos:pos+2])[0]; pos+=2
    cp=[None]*cp_count
    i=1
    while i<cp_count:
        tag=data[pos]; pos+=1
        if tag==1:
            l=struct.unpack('>H',data[pos:pos+2])[0]; pos+=2; v=data[pos:pos+l].decode('utf-8','replace'); pos+=l; cp[i]=('Utf8',v); i+=1
        elif tag in (3,4):
            raw=data[pos:pos+4]
            v=struct.unpack('>i',raw)[0] if tag==3 else struct.unpack('>f',raw)[0]
            pos+=4; cp[i]=(tag,v); i+=1
        elif tag in (5,6):
            raw=data[pos:pos+8]
            v=struct.unpack('>q',raw)[0] if tag==5 else struct.unpack('>d',raw)[0]
            pos+=8; cp[i]=(tag,v); i+=2
        elif tag in (7,8,16,19,20):
            idx=struct.unpack('>H',data[pos:pos+2])[0]; pos+=2; cp[i]=(tag,idx); i+=1
        elif tag in (9,10,11,12,17,18):
            a=struct.unpack('>H',data[pos:pos+2])[0]; b=struct.unpack('>H',data[pos+2:pos+4])[0]; pos+=4; cp[i]=(tag,a,b); i+=1
        elif tag==15:
            a=data[pos]; b=struct.unpack('>H',data[pos+1:pos+3])[0]; pos+=3; cp[i]=(tag,a,b); i+=1
        else:
            raise ValueError(f'unknown tag {tag}')
    return data,pos,cp

def resolve(cp, idx):
    if idx<=0 or idx>=len(cp) or cp[idx] is None: return f'#{idx}'
    v=cp[idx]
    if v[0]=='Utf8': return v[1]
    tag=v[0]
    if tag==7: return resolve(cp,v[1])
    if tag==8: return repr(resolve(cp,v[1]))
    if tag==12: return resolve(cp,v[1])+':'+resolve(cp,v[2])
    if tag in (9,10,11): return resolve(cp,v[1])+'.'+resolve(cp,v[2])
    if tag in (3,4): return str(v[1])
    return str(v)

def disasm(path, want):
    data,pos0,cp=load(path)
    pos=pos0+6
    n=struct.unpack('>H',data[pos:pos+2])[0]; pos+=2+2*n
    nf=struct.unpack('>H',data[pos:pos+2])[0]; pos+=2
    for _ in range(nf):
        pos+=6
        m=struct.unpack('>H',data[pos:pos+2])[0]; pos+=2
        for __ in range(m):
            pos+=2; l=struct.unpack('>I',data[pos:pos+4])[0]; pos+=4+l
    nm=struct.unpack('>H',data[pos:pos+2])[0]; pos+=2
    for _ in range(nm):
        pos+=2
        ni=struct.unpack('>H',data[pos:pos+2])[0]; pos+=2
        di=struct.unpack('>H',data[pos:pos+2])[0]; pos+=2
        m=struct.unpack('>H',data[pos:pos+2])[0]; pos+=2
        name=resolve(cp,ni); desc=resolve(cp,di)
        full=f'{name}{desc}'
        attrs={}
        for __ in range(m):
            an=struct.unpack('>H',data[pos:pos+2])[0]; pos+=2
            l=struct.unpack('>I',data[pos:pos+4])[0]; pos+=4
            attrs[resolve(cp,an)]=data[pos:pos+l]
            pos+=l
        if full in want or name in want:
            print(f'=== {path}:{full} ===')
            if 'Code' not in attrs:
                print('  (no code)')
                continue
            code=attrs['Code']
            code_len=struct.unpack('>I',code[4:8])[0]
            bc=code[8:8+code_len]
            print(f'  len={code_len}')
            pc=0; count=0
            while pc < len(bc):
                opcode=bc[pc]
                if opcode not in OP:
                    print(f'  {pc:4d}: <0x{opcode:02x} ?>'); pc+=1
                else:
                    nm2,fmt=OP[opcode]
                    operands=''; oplen=1
                    if fmt=='B':
                        v=bc[pc+1]; operands=str(v)
                        if nm2=='ldc': operands+=f' ({resolve(cp,v)})'
                        oplen=2
                    elif fmt=='b':
                        raw=bc[pc+1]; operands=str(raw-256 if raw>127 else raw); oplen=2
                    elif fmt=='H':
                        v=struct.unpack('>H',bc[pc+1:pc+3])[0]; operands=f'#{v} ({resolve(cp,v)})'; oplen=3
                    elif fmt in ('h','s'):
                        operands=str(struct.unpack('>h',bc[pc+1:pc+3])[0]); oplen=3
                    elif fmt=='BB':
                        operands=f'{bc[pc+1]} {bc[pc+2]}'; oplen=3
                    elif fmt=='H2':
                        v=struct.unpack('>H',bc[pc+1:bc+3] if False else bc[pc+1:pc+3])[0]; operands=f'#{v} ({resolve(cp,v)}) +{bc[pc+3]}'; oplen=5
                    print(f'  {pc:4d}: {nm2} {operands}')
                    pc+=oplen
                count+=1
                if count>400:
                    print('  ... truncated'); break

if __name__=='__main__':
    disasm(sys.argv[1], sys.argv[2:])
