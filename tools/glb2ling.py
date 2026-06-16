#!/usr/bin/env python3
"""glb2ling — convert a binary glTF (.glb) into a Ling draw module.

Extracts mesh geometry (positions/normals/triangles) in world space (bind/rest
pose: each mesh baked by its node's world transform), optionally the skin's
skeleton (joints + bones), decimates to a triangle budget via vertex clustering,
and emits a `.ling` file with a flat triangle list + a `วาด<Name>(cx,cy,cz,sc,yaw)`
draw function (and `วาด<Name>ริก(...)` for the rig skeleton).

Usage:
  python tools/glb2ling.py model/queen.glb game/queen_model.ling --name Queen \
      --tris 900 --rig --scale 1.0

Notes:
- glTF has no per-pixel z-buffer in the target engine; decimate aggressively.
- Animations are NOT exported (the source files contain none); the rig is the
  rest pose so the engine can pose/animate it procedurally if desired.
"""
import struct, json, sys, math, argparse

C_SIZE = {5120:1,5121:1,5122:2,5123:2,5125:4,5126:4}
C_FMT  = {5120:'b',5121:'B',5122:'h',5123:'H',5125:'I',5126:'f'}
T_COUNT= {'SCALAR':1,'VEC2':2,'VEC3':3,'VEC4':4,'MAT4':16}

def load_glb(path):
    with open(path,'rb') as f: data=f.read()
    magic,ver,_=struct.unpack('<4sII',data[:12])
    assert magic==b'glTF' and ver==2, "not a glTF2 binary"
    off=12; gltf=None; binc=b''
    while off<len(data):
        clen,ctype=struct.unpack('<II',data[off:off+8])
        chunk=data[off+8:off+8+clen]
        if ctype==0x4E4F534A: gltf=json.loads(chunk)
        elif ctype==0x004E4942: binc=chunk
        off+=8+clen
    return gltf,binc

def read_accessor(gltf,binc,idx):
    acc=gltf['accessors'][idx]
    bv=gltf['bufferViews'][acc['bufferView']]
    comp=acc['componentType']; ncomp=T_COUNT[acc['type']]
    csize=C_SIZE[comp]; fmt=C_FMT[comp]
    base=bv.get('byteOffset',0)+acc.get('byteOffset',0)
    count=acc['count']; elem=csize*ncomp
    stride=bv.get('byteStride', elem)
    if stride==elem:                       # contiguous → one bulk unpack (fast)
        allv=struct.unpack('<'+fmt*(ncomp*count), binc[base:base+elem*count])
        if ncomp==1: return list(allv)
        return [allv[i*ncomp:(i+1)*ncomp] for i in range(count)]
    out=[]
    for i in range(count):
        p=base+i*stride
        vals=struct.unpack_from('<'+fmt*ncomp, binc, p)
        out.append(vals if ncomp>1 else vals[0])
    return out

# ---- 4x4 matrix math (column-major like glTF) -------------------------------
def mat_id(): return [1,0,0,0, 0,1,0,0, 0,0,1,0, 0,0,0,1]
def mat_mul(a,b):
    r=[0]*16
    for c in range(4):
        for rw in range(4):
            s=0.0
            for k in range(4): s+=a[k*4+rw]*b[c*4+k]
            r[c*4+rw]=s
    return r
def trs_matrix(t,q,s):
    x,y,z,w=q
    xx,yy,zz=x*x,y*y,z*z; xy,xz,yz=x*y,x*z,y*z; wx,wy,wz=w*x,w*y,w*z
    sx,sy,sz=s
    return [
        (1-2*(yy+zz))*sx, (2*(xy+wz))*sx, (2*(xz-wy))*sx, 0,
        (2*(xy-wz))*sy, (1-2*(xx+zz))*sy, (2*(yz+wx))*sy, 0,
        (2*(xz+wy))*sz, (2*(yz-wx))*sz, (1-2*(xx+yy))*sz, 0,
        t[0],t[1],t[2],1]
def node_local(n):
    if 'matrix' in n: return list(n['matrix'])
    t=n.get('translation',[0,0,0]); q=n.get('rotation',[0,0,0,1]); s=n.get('scale',[1,1,1])
    return trs_matrix(t,q,s)
def xform(m,p):
    x,y,z=p
    return (m[0]*x+m[4]*y+m[8]*z+m[12],
            m[1]*x+m[5]*y+m[9]*z+m[13],
            m[2]*x+m[6]*y+m[10]*z+m[14])

def world_matrices(gltf):
    nodes=gltf['nodes']; world={}
    roots=[]
    if 'scenes' in gltf:
        roots=gltf['scenes'][gltf.get('scene',0)].get('nodes',list(range(len(nodes))))
    else: roots=list(range(len(nodes)))
    def rec(i,parent):
        m=mat_mul(parent,node_local(nodes[i]))
        world[i]=m
        for c in nodes[i].get('children',[]): rec(c,m)
    for r in roots: rec(r,mat_id())
    return world

def mat_color(gltf,prim):
    mi=prim.get('material')
    if mi is None: return (200,200,210)
    mat=gltf['materials'][mi]
    pbr=mat.get('pbrMetallicRoughness',{})
    bc=pbr.get('baseColorFactor',[0.8,0.8,0.82,1])
    return (max(0,min(255,int(bc[0]*255))),max(0,min(255,int(bc[1]*255))),max(0,min(255,int(bc[2]*255))))

def collect_tris(gltf,binc):
    world=world_matrices(gltf)
    tris=[]   # each: (p0,p1,p2,(r,g,b))
    for ni,n in enumerate(gltf['nodes']):
        if 'mesh' not in n: continue
        m=world.get(ni,mat_id())
        for prim in gltf['meshes'][n['mesh']]['primitives']:
            attrs=prim['attributes']
            if 'POSITION' not in attrs: continue
            pos=read_accessor(gltf,binc,attrs['POSITION'])
            col=mat_color(gltf,prim)
            if 'indices' in prim:
                idx=read_accessor(gltf,binc,prim['indices'])
            else:
                idx=list(range(len(pos)))
            wp=[xform(m,p) for p in pos]
            for t in range(0,len(idx)-2,3):
                a,b,c=idx[t],idx[t+1],idx[t+2]
                tris.append((wp[a],wp[b],wp[c],col))
    return tris

def bounds(tris):
    lo=[1e30]*3; hi=[-1e30]*3
    for tr in tris:
        for p in tr[:3]:
            for k in range(3):
                lo[k]=min(lo[k],p[k]); hi[k]=max(hi[k],p[k])
    return lo,hi

def decimate(tris, budget, grid0=64):
    """Vertex-cluster decimation (Rossignac-style): partition space into a grid,
    collapse each cell's vertices to their **average** position (not the cell
    centre → smooth vector planes, not voxel blocks), remap+dedup triangles, and
    tune the grid to land near `budget` triangles."""
    if not tris: return tris,(([0,0,0],[0,0,0]))
    lo,hi=bounds(tris)
    ext=max(hi[k]-lo[k] for k in range(3)) or 1.0
    def cluster(grid):
        cell=ext/grid
        def keyof(p): return (int((p[0]-lo[0])/cell),int((p[1]-lo[1])/cell),int((p[2]-lo[2])/cell))
        acc={}                                  # cell -> [sumx,sumy,sumz,count]
        for (a,b,c,_col) in tris:
            for p in (a,b,c):
                k=keyof(p); e=acc.get(k)
                if e: e[0]+=p[0]; e[1]+=p[1]; e[2]+=p[2]; e[3]+=1
                else: acc[k]=[p[0],p[1],p[2],1]
        rep={k:(e[0]/e[3],e[1]/e[3],e[2]/e[3]) for k,e in acc.items()}   # averaged representative
        out=[]; seen=set()
        for (a,b,c,col) in tris:
            ka=keyof(a); kb=keyof(b); kc=keyof(c)
            if ka==kb or kb==kc or ka==kc: continue
            key=(ka,kb,kc,col)
            if key in seen: continue
            seen.add(key)
            out.append((rep[ka],rep[kb],rep[kc],col))
        return out
    grid=grid0
    res=cluster(grid)
    for _ in range(10):
        if len(res)>budget*1.15 and grid>6: grid=int(grid*0.82); res=cluster(grid)
        elif len(res)<budget*0.7: grid=int(grid*1.22); res=cluster(grid)
        else: break
    return res,(lo,hi)

def collect_rig(gltf):
    """Return (joints[(x,y,z)], bones[(i,j)]) for skin[0] in world rest pose."""
    if not gltf.get('skins'): return [],[]
    world=world_matrices(gltf)
    skin=gltf['skins'][0]; joints=skin['joints']
    jidx={n:k for k,n in enumerate(joints)}
    pts=[]
    for n in joints:
        m=world.get(n,mat_id()); pts.append((m[12],m[13],m[14]))
    bones=[]
    for n in joints:
        for c in gltf['nodes'][n].get('children',[]):
            if c in jidx: bones.append((jidx[n],jidx[c]))
    return pts,bones

def normalize(tris, target_h, lo, hi, flip=False, scale=1.0):
    """Center on origin (x,z), feet at y=0, scale to target height (× scale). With
    flip the model extends toward -Y (head at -height) for screen-up = -Y engines."""
    cx=(lo[0]+hi[0])*0.5; cz=(lo[2]+hi[2])*0.5; fy=lo[1]
    h=(hi[1]-lo[1]) or 1.0; s=(target_h/h)*scale; sy = -s if flip else s
    def nz(p): return ((p[0]-cx)*s,(p[1]-fy)*sy,(p[2]-cz)*s)
    return [(nz(a),nz(b),nz(c),col) for (a,b,c,col) in tris], (cx,cz,fy,s,sy)

def arms_down(tris, H):
    """Rotate arm vertices around the shoulder from the GLB T-pose (arms out along
    ±X) to hanging at the sides (down toward +Y). Weighted by an arm mask so the
    shoulder stays put and the hand swings fully. Z is preserved so the runtime
    fore/aft swing + elbow bend still work. Coords are normalized+flipped."""
    ys = -0.72*H; torso = 0.13*H; yc = -0.68*H
    out=[]
    for (a,b,c,col) in tris:
        nv=[]
        for p in (a,b,c):
            x,y,z=p; ax=abs(x)
            yb=max(0.0,min(1.0,1.0-abs(y-yc)/(0.34*H)))
            aw=max(0.0,min(1.0,(ax-torso)/(0.06*H)))*yb   # steep ramp → whole arm rotates as one (hangs straight)
            if aw<=0.0: nv.append((x,y,z)); continue
            side=1.0 if x>=0 else -1.0
            sx=side*0.15*H; dx=x-sx; dy=y-ys
            th=side*(math.pi*0.52)*aw                      # ~94° → straight down at the side
            cth=math.cos(th); sth=math.sin(th)
            nv.append((sx+dx*cth-dy*sth, ys+dx*sth+dy*cth, z))
        out.append((nv[0],nv[1],nv[2],col))
    return out

def tint_clothes(tris, tint, target_h, scale):
    """Recolor the torso/legs band (clothing) toward `tint` (RGB 0..255), keeping the
    head and feet original. Coords are normalized+flipped: feet 0 → head -height."""
    H=target_h*scale
    out=[]
    for (a,b,c,col) in tris:
        cy=(a[1]+b[1]+c[1])/3.0
        if -0.82*H < cy < -0.06*H:    # clothing band (above feet, below neck)
            col=(int(tint[0]*0.72+col[0]*0.28), int(tint[1]*0.72+col[1]*0.28), int(tint[2]*0.72+col[2]*0.28))
        out.append((a,b,c,col))
    return out

def emit(name, tris, rig_pts, rig_bones, norm_for_rig, out_path, sway=False):
    # Emit LITERAL draw calls (no big list variable): the interpreter clones a
    # list variable on every read, so list_get on a 16k-element list is O(n²).
    # Baking literals keeps each triangle O(1); yaw rotation is applied inline.
    # sway: add a `sway` param → height-weighted horizontal lean (head/arms move).
    def fnum(v): return f"{v:.4f}"
    def vexpr(p):
        if sway:
            xs=f"({fnum(p[0])}+sway*{fnum(abs(p[1]))})"   # x offset ∝ height (feet 0 → head most)
        else:
            xs=fnum(p[0])
        return (f"cx+({xs}*cs+{fnum(p[2])}*sn)*sc",
                f"cy+{fnum(p[1])}*sc",
                f"cz+({fnum(p[2])}*cs-{xs}*sn)*sc")
    sig = "cx, cy, cz, sc, yaw, sway" if sway else "cx, cy, cz, sc, yaw"
    lines=[]
    lines.append(f"# {name} — auto-generated by tools/glb2ling.py ({len(tris)} triangles). Do not hand-edit.")
    lines.append(f"# วาด{name}({sig}): centered on x/z, feet at cy, height≈sc; yaw rotates about Y.")
    lines.append(f"ฟังก์ชัน วาด{name}({sig}) {{")
    lines.append(f"    bind cs = โคไซน์(yaw);  bind sn = ไซน์(yaw)")
    last=None
    for (a,b,c,col) in sorted(tris, key=lambda t:t[3]):   # group by color → fewer สีดินสอ
        if col!=last:
            lines.append(f"    สีดินสอ({col[0]}, {col[1]}, {col[2]})"); last=col
        ax,ay,az=vexpr(a); bx,by,bz=vexpr(b); cx2,cy2,cz2=vexpr(c)
        lines.append(f"    วาดสามเหลี่ยม3มิติ({ax}, {ay}, {az}, {bx}, {by}, {bz}, {cx2}, {cy2}, {cz2})")
    lines.append(f"    flush_3d()")
    lines.append(f"}}")
    if rig_pts:
        cxn,czn,fyn,s,sy=norm_for_rig
        def nz(p): return ((p[0]-cxn)*s,(p[1]-fyn)*sy,(p[2]-czn)*s)
        npz=[nz(p) for p in rig_pts]
        lines.append("")
        lines.append(f"# วาด{name}ริก(cx,cy,cz, sc, yaw, r,g,b): rest-pose skeleton (bones as glowing lines)")
        lines.append(f"ฟังก์ชัน วาด{name}ริก(cx, cy, cz, sc, yaw, r, g, b) {{")
        lines.append(f"    โหมดผสม(1);  สีดินสอ(r, g, b)")
        lines.append(f"    bind cs = โคไซน์(yaw);  bind sn = ไซน์(yaw)")
        for (i,j) in rig_bones:
            ax,ay,az=vexpr(npz[i]); bx,by,bz=vexpr(npz[j])
            lines.append(f"    วาดเส้น3มิติ({ax}, {ay}, {az}, {bx}, {by}, {bz})")
        lines.append(f"    flush_3d();  โหมดผสม(0)")
        lines.append(f"}}")
    with open(out_path,'w',encoding='utf-8') as f: f.write("\n".join(lines)+"\n")
    return len(tris), len(rig_bones)

def write_lmesh(tris, height, path):
    """Binary mesh for the engine's mesh_load/mesh_draw (native-res, fast Rust draw).
    Layout (little-endian): 'LMSH', u32 version=1, f32 height, u32 ntri,
    then ntri × (9 f32 positions + 3 u8 rgb)."""
    import struct
    with open(path,'wb') as f:
        f.write(b'LMSH'); f.write(struct.pack('<I',1)); f.write(struct.pack('<f',height)); f.write(struct.pack('<I',len(tris)))
        cl=lambda v:max(0,min(255,int(v)))
        for (a,b,c,col) in tris:
            f.write(struct.pack('<9f', a[0],a[1],a[2], b[0],b[1],b[2], c[0],c[1],c[2]))
            f.write(struct.pack('<3B', cl(col[0]),cl(col[1]),cl(col[2])))

def main():
    ap=argparse.ArgumentParser()
    ap.add_argument('glb'); ap.add_argument('out')
    ap.add_argument('--name',required=True)
    ap.add_argument('--tris',type=int,default=900)
    ap.add_argument('--height',type=float,default=1.0)
    ap.add_argument('--rig',action='store_true')
    ap.add_argument('--flipy',action='store_true',help='extend toward -Y (engine screen-up is -Y)')
    ap.add_argument('--sway',action='store_true',help='add a sway param (height-weighted lean → head/arms move)')
    ap.add_argument('--scale',type=float,default=1.0,help='multiply baked size (e.g. 3 = 3× bigger)')
    ap.add_argument('--tint',type=str,default=None,help='recolor clothing band toward "R,G,B"')
    ap.add_argument('--native',action='store_true',help='skip decimation (full GLB resolution; for .lmesh)')
    ap.add_argument('--armsdown',action='store_true',help='rotate T-pose arms to hang at the sides')
    args=ap.parse_args()
    print(f"loading {args.glb} ...")
    gltf,binc=load_glb(args.glb)
    print("collecting triangles ...")
    tris=collect_tris(gltf,binc)
    print(f"  raw triangles: {len(tris)}")
    if args.native:
        lo,hi=bounds(tris);  print("  native res (no decimation)")
    else:
        tris,(lo,hi)=decimate(tris,args.tris);  print(f"  decimated: {len(tris)}")
    tris,norm=normalize(tris,args.height,lo,hi,flip=args.flipy,scale=args.scale)
    if args.tint:
        tint=tuple(float(v) for v in args.tint.split(','))
        tris=tint_clothes(tris,tint,args.height,args.scale)
        print(f"  tinted clothing -> {tint}")
    if args.armsdown:
        tris=arms_down(tris, args.height*args.scale)
        print("  arms rotated down to sides")
    if args.out.endswith('.lmesh'):                       # binary mesh for engine mesh_load/mesh_draw
        write_lmesh(tris, args.height*args.scale, args.out)
        print(f"wrote {args.out}: {len(tris)} tris (binary .lmesh, h={args.height*args.scale:.2f})")
        return
    rig_pts,rig_bones=([],[])
    if args.rig:
        rig_pts,rig_bones=collect_rig(gltf)
        print(f"  rig: {len(rig_pts)} joints, {len(rig_bones)} bones")
    nt,nb=emit(args.name,tris,rig_pts,rig_bones,norm,args.out,sway=args.sway)
    print(f"wrote {args.out}: {nt} tris, {nb} bones")

if __name__=='__main__': main()
