//! Opt-in check of the app's real SolidCraft → Blender benchtop export.
use std::collections::HashMap;
use std::process::Command;

use cabinet_core::{Cabinet, CabinetKind};
use cabinet_ffi::{JobItems, write_blender_job};

#[test]
fn a_blender_scene_keeps_the_benchtop_bounds_sink_and_tap_holes() {
    check_scene(false, true, None);
}

#[test]
fn a_blender_scene_keeps_mitred_returns_packers_and_the_contrasting_shadow_band() {
    check_scene(true, true, None);
}

#[test]
fn a_blender_scene_keeps_rear_supports_and_a_single_seen_back_without_waterfalls() {
    check_scene(true, false, None);
}

#[test]
fn smartstone_matt_and_suede_use_local_full_slabs_without_vein_relief() {
    check_scene(true, true, Some("smartstone.sintered.calacatta-verde.20"));
    check_scene(true, false, Some("smartstone.sintered.pietra-scuro.20"));
}

fn check_scene(built_up: bool, falls: bool, stone: Option<&str>) {
    let (Some(cli), Some(blender)) = (
        std::env::var_os("SOLIDCRAFT_CLI"),
        std::env::var_os("BLENDER"),
    ) else {
        eprintln!("SOLIDCRAFT_CLI and BLENDER required: skipped");
        return;
    };
    let dir = std::env::temp_dir().join(format!(
        "cabinet-bench-check-{}-{built_up}-{falls}-{}",
        std::process::id(),
        stone.unwrap_or("generic")
    ));
    std::fs::create_dir_all(dir.join("config")).unwrap();
    let mut sink = Cabinet::new("S01", CabinetKind::Sink);
    sink.width = 900.0;
    let mut items = Vec::new();
    sink.cladding.back = built_up;
    items.push(serde_json::to_string(&sink).unwrap());
    if built_up {
        let mut base = Cabinet::new("B02", CabinetKind::Base);
        base.width = 900.0;
        base.cladding.back = true;
        items.push(serde_json::to_string(&base).unwrap());
    }
    items.push(serde_json::json!({
        "type":"benchtop","id":"BT1","board":stone.unwrap_or(""),"waterfall_left":falls,"waterfall_right":built_up&&falls,
        "end_radius":if built_up {0} else {60},"back_overhang":if built_up {300} else {0},
        "edge":if built_up {serde_json::json!({"type":"mitred_apron","depth":if falls {60} else {40}})} else {serde_json::json!({"type":"pencil_round"})},
        "thickness":if built_up {20} else {33},
        "build_up":{"shadow_height":15,"edge_tape":{"id":"accent","colour":"#aa7722","thickness":1}},
        "cutouts":[{"over":"S01","kind":"sink","width":600,"depth":if built_up {500} else {400},"tap_hole":35,"tap_hole_behind":55}]
    }).to_string());
    let job = JobItems {
        items_json: items,
        placed: Vec::new(),
    };
    write_blender_job(
        job,
        concat!(env!("CARGO_MANIFEST_DIR"), "/../../catalog").into(),
        String::new(),
        HashMap::new(),
        dir.to_string_lossy().into(),
    )
    .unwrap();
    let output = Command::new(cli)
        .arg("run")
        .arg(dir.join("build.json"))
        .arg("--out")
        .arg(dir.join("job.obj"))
        .arg("--quiet")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let script = r#"
import json, os, sys
import bpy
from mathutils import Vector
sys.path.insert(0, os.environ['SCENE_SCRIPT_DIR'])
from build_scene import build
folder = os.environ['BENCH_CHECK_DIR']
path = lambda name: os.path.join(folder, name)
build(path('job.json'), path('job.obj'), path('check.blend'), path('cache'), None)
job = json.load(open(path('job.json')))
for part in job['parts']:
    if part['role'] != 'Benchtop':
        continue
    ob = bpy.data.objects[part['name']]
    points = [ob.matrix_world @ Vector(c) for c in ob.bound_box]
    for axis in range(3):
        assert abs(min(p[axis] for p in points) * 1000 - part['min'][axis]) < 0.05, part['name']
        assert abs(max(p[axis] for p in points) * 1000 - part['max'][axis]) < 0.05, part['name']
top = bpy.data.objects['BT1.Benchtop1']
inverse = top.matrix_world.inverted()
def hits(x, y):
    origin = inverse @ Vector((x / 1000, y / 1000, 1.1))
    direction = inverse.to_3x3() @ Vector((0, 0, -1))
    return top.ray_cast(origin, direction)[0]
assert not hits(450, 242), 'sink opening filled in'
tap_y = 597 if os.environ['BENCH_BUILT_UP'] == '1' else 497
assert not hits(450, tap_y), 'tap hole filled in'
assert hits(100, 100), 'solid benchtop missing'
if os.environ['BENCH_BUILT_UP'] == '1':
    assert 'BT1.Apron1' in bpy.data.objects
    assert 'BT1.ShadowRail1' in bpy.data.objects
    assert 'BT1.ShadowRail1Back' in bpy.data.objects
    assert 'BT1.ShadowBand1Back' in bpy.data.objects
    assert 'BT1.Apron1Back' in bpy.data.objects
    back = bpy.data.objects['S01.SeenBack']
    assert 'B02.SeenBack' not in bpy.data.objects, 'one rear board per cabinet survived'
    assert abs(back.dimensions.x * 1000 - 1800) < 0.1
    rail = bpy.data.objects['BT1.ShadowRail1Back']
    vertices = [rail.matrix_world @ v.co for v in rail.data.vertices]
    assert max(v.y for v in vertices) * 1000 < 600, 'rear rail follows seating overhang'
    assert abs(min(v.z for v in vertices) * 1000 - 870) < 0.1, 'rear rail floats above cabinet'
    if os.environ['BENCH_FALLS'] == '0':
        assert 'BT1.ShadowRail1Left' in bpy.data.objects
        assert 'BT1.ShadowBand1Right' in bpy.data.objects
        assert 'BT1.Waterfall1L' not in bpy.data.objects

    cap = bpy.data.objects['BT1.Underlining1']
    inv = cap.matrix_world.inverted()
    def lower_hit(x, y):
        return cap.ray_cast(inv @ Vector((x/1000,y/1000,0)), inv.to_3x3() @ Vector((0,0,1)))
    hit, position, _, _ = lower_hit(900,750)
    assert hit, 'seating pocket is still open underneath'
    assert abs((cap.matrix_world @ position).z*1000-885) < 0.1, 'lower stone is not flush with the built-up edge'
    assert not lower_hit(450,530)[0], 'partial sink opening blocked by the lower skin'
    assert not lower_hit(450,597)[0], 'tap hole blocked by the lower skin'
    assert not lower_hit(900,100)[0], 'lower skin extends under the cabinet interior'
    cap_meta = next(p for p in job['parts'] if p['name'] == cap.name)
    rail_points = [rail.matrix_world @ v.co for v in rail.data.vertices]
    assert abs(max(v.z for v in rail_points)*1000 - cap_meta['min'][2]) < 0.1, 'rail does not bear on underside stone'
    for side in ['', 'Back']:
        for end in ['Left','Right']:
            ob = bpy.data.objects[f'BT1.ShadowBand1{side}End{end}']
            assert len(ob.data.polygons)>0, 'corner band geometry missing'
            rgb = ob.active_material.node_tree.nodes.get('Principled BSDF').inputs['Base Color'].default_value
            assert rgb[0]>rgb[1]>rgb[2], f'raw white corner at {ob.name}'
    band = bpy.data.objects['BT1.ShadowBand1']
    colour = band.active_material.node_tree.nodes.get('Principled BSDF').inputs['Base Color'].default_value
    assert colour[0] > colour[1] > colour[2], f'contrast band lost its material: {colour}'
else:
    origin = inverse @ Vector((0.1, -0.037, 1.1))
    hit, position, _, _ = top.ray_cast(origin, inverse.to_3x3() @ Vector((0, 0, -1)))
    assert hit, 'pencil-round edge missing'
    height = (top.matrix_world @ position).z
    expected = (903 - 2 + (2**2 - 1**2)**0.5) / 1000
    assert abs(height - expected) < 0.0002, f'pencil round is square or misplaced: {height} vs {expected}'
if os.environ.get('STONE_PRODUCT'):
    material = top.active_material
    nodes = material.node_tree.nodes
    suede = 'pietra-scuro' in os.environ['STONE_PRODUCT']
    bsdf = nodes.get('Principled BSDF')
    assert abs(bsdf.inputs['Roughness'].default_value - (0.48 if suede else 0.32)) < 0.001
    images = [n.image for n in nodes if n.type == 'TEX_IMAGE']
    assert len(images) == 1 and images[0].size[0] == 2500, 'full slab image missing'
    assert images[0].packed_file, 'slab texture not packed into portable blend file'
    bumps = [n for n in nodes if n.type == 'BUMP']
    assert len(bumps) == (1 if suede else 0), 'veins have been turned into relief'
    if suede:
        assert abs(bumps[0].inputs['Distance'].default_value - 0.00001) < 1e-8
        assert any(n.type == 'TEX_NOISE' for n in nodes)
    meta = job['boards'][os.environ['STONE_PRODUCT']]
    assert meta['sheet'] == [3200,1600]
    for poly in top.data.polygons:
        if poly.normal.z < 0.99: continue
        loops = list(poly.loop_indices)
        a = loops[0]
        for c in loops[1:]:
            va, vc = [top.matrix_world @ top.data.vertices[top.data.loops[k].vertex_index].co for k in [a,c]]
            ua, uc = [top.data.uv_layers['panel'].data[k].uv for k in [a,c]]
            assert abs((uc.x-ua.x) - (vc.x-va.x)/3.2) < 0.0001, 'incorrect full slab length scale'
            assert abs((uc.y-ua.y) + (vc.y-va.y)/1.6) < 0.0001, 'incorrect full slab width scale'
assert not any(o.name.startswith('Benchtop ') for o in bpy.data.objects), 'scene-only top duplicated the real top'
print('benchtop scene checks passed')
"#;
    let output = Command::new(blender)
        .args([
            "-b",
            "--factory-startup",
            "--python-exit-code",
            "1",
            "--python-expr",
            script,
        ])
        .env("BENCH_CHECK_DIR", &dir)
        .env("STONE_PRODUCT", stone.unwrap_or(""))
        .env("BENCH_FALLS", if falls { "1" } else { "0" })
        .env("BENCH_BUILT_UP", if built_up { "1" } else { "0" })
        .env(
            "SCENE_SCRIPT_DIR",
            concat!(env!("CARGO_MANIFEST_DIR"), "/../../blender"),
        )
        .env("BLENDER_USER_CONFIG", dir.join("config"))
        .output()
        .unwrap();
    assert!(
        output.status.success()
            && String::from_utf8_lossy(&output.stdout).contains("benchtop scene checks passed"),
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        !dir.join("config/userpref.blend").exists(),
        "scene generation overwrote Blender preferences"
    );
    std::fs::remove_dir_all(dir).unwrap();
}
