"""Brings a Cabinet Studio job into Revit: every part a Casework DirectShape, in a material with its
decor's image. Revit reads no file format that carries textures, so this builds them with its API.

Runs inside Revit, from `Import into Revit.dyn` in Dynamo, with this job's folder beside it. The
helpers above `main` need no Revit: revit/test_import.py checks them.
"""
import json
import os

try:
    import clr

    IN_REVIT = True
except ImportError:
    IN_REVIT = False

FEET_PER_MM = 1 / 304.8
PREFIX = "CS "


def parse_mtl(path):
    """Material name -> diffuse colour, 0 to 1."""
    colours, name = {}, None
    if not os.path.exists(path):
        return colours
    with open(path, encoding="utf-8") as f:
        for line in f:
            parts = line.split()
            if len(parts) >= 2 and parts[0] == "newmtl":
                name = parts[1]
            elif len(parts) == 4 and parts[0] == "Kd" and name:
                colours[name] = tuple(float(v) for v in parts[1:])
    return colours


def parse_obj(path):
    """(vertices in mm, bodies): each body a name, its OBJ material, and triangles of vertex indices."""
    verts, bodies, body = [], [], None
    with open(path, encoding="utf-8") as f:
        for line in f:
            if line.startswith("v "):
                verts.append(tuple(float(v) for v in line.split()[1:4]))
            elif line.startswith("o "):
                body = {"name": line[2:].strip(), "material": None, "triangles": []}
                bodies.append(body)
            elif line.startswith("usemtl ") and body:
                body["material"] = line[7:].strip()
            elif line.startswith("f ") and body:
                ring = [int(p.split("/")[0]) - 1 for p in line.split()[1:]]
                for i in range(1, len(ring) - 1):
                    body["triangles"].append((ring[0], ring[i], ring[i + 1]))
    return verts, bodies


def hex_colour(text):
    """"#RRGGBB" as 0 to 255, or None."""
    if not text or len(text.lstrip("#")) != 6:
        return None
    t = text.lstrip("#")
    return tuple(int(t[i : i + 2], 16) for i in (0, 2, 4))




def grain_is_upright(part):
    """Does the grain run up the face Revit maps? Revit's V is up a wall, and along Y on a flat panel."""
    grain, thick = part.get("grain"), part.get("thickness")
    if grain is None or grain == "z":
        return True
    return grain == "y" and thick == "z"


def picture_size(board):
    """(width, height) in mm of the board's picture, and whether its long side is its height."""
    sheet = board.get("sheet") or [2400.0, 1200.0]
    pixels = board.get("pixels") or []
    w, h = (pixels + [0, 0])[:2] if len(pixels) == 2 else (1, 2)
    long_side = float(sheet[0])
    short = long_side * (min(w, h) / max(w, h)) if max(w, h) else long_side / 2
    return ((short, long_side) if h >= w else (long_side, short)), h >= w


def material_of(body, part, boards, kd):
    """What a body wears: a description of one Revit material. Bodies sharing a key share it."""
    fallback = tuple(round(v * 255) for v in kd.get(body["material"], (0.8, 0.77, 0.7)))
    if part is None:
        # Supplier hardware, in the colour SolidCraft gave it, as metal.
        key = "Hardware %02X%02X%02X" % fallback
        return {"key": key, "name": PREFIX + key, "colour": fallback, "roughness": 0.3, "metal": True}
    paint = part.get("paint")
    if paint:
        name = "%s %s" % (paint["name"], paint["finish"])
        return {
            "key": "paint " + name,
            "name": PREFIX + name,
            "colour": hex_colour(paint["colour"]) or fallback,
            "roughness": paint.get("roughness", 0.5),
        }
    board = boards.get(part.get("board") or "")
    if not board:
        key = "Generic %02X%02X%02X" % fallback
        return {"key": key, "name": PREFIX + key, "colour": fallback, "roughness": 0.5}
    name = " ".join(x for x in (board.get("decor"), board.get("finish")) if x)
    look = board.get("look") or {}
    # One material per decor and finish, whatever thicknesses the job uses it in.
    out = {
        "key": name,
        "name": PREFIX + name,
        "colour": hex_colour(board.get("colour")) or fallback,
        "roughness": look.get("roughness", 0.5),
    }
    if board.get("image"):
        size, picture_upright = picture_size(board)
        turn = board.get("grained") and picture_upright != grain_is_upright(part)
        out.update(image=board["image"], size=size, angle=90.0 if turn else 0.0)
        if turn:
            # Revit turns a picture per material, not per face.
            out["key"] += " across"
            out["name"] += " across"
    return out


def set_distance(bitmap, name, mm):
    from Autodesk.Revit.DB import UnitTypeId, UnitUtils

    prop = bitmap.FindByName(name)
    if prop is not None:
        prop.Value = UnitUtils.Convert(mm, UnitTypeId.Millimeters, prop.GetUnitTypeId())


def set_bitmap(asset, slot, path, spec):
    """A picture in one of a Generic appearance's slots, at the size it covers on the board."""
    prop = asset.FindByName(slot)
    if prop is None:
        raise ValueError("this appearance has no " + slot)
    if prop.NumberOfConnectedProperties == 0:
        prop.AddConnectedAsset("UnifiedBitmap")
    bitmap = prop.GetSingleConnectedAsset()
    bitmap.FindByName("unifiedbitmap_Bitmap").Value = path
    set_distance(bitmap, "texture_RealWorldScaleX", spec["size"][0])
    set_distance(bitmap, "texture_RealWorldScaleY", spec["size"][1])
    set_distance(bitmap, "texture_RealWorldOffsetX", 0.0)
    set_distance(bitmap, "texture_RealWorldOffsetY", 0.0)
    angle = bitmap.FindByName("texture_WAngle")
    if angle is not None:
        angle.Value = spec["angle"]


def make_material(doc, spec, folder, base, notes):
    from Autodesk.Revit.DB import AppearanceAssetElement, Color, ElementId, FilteredElementCollector, Material
    from Autodesk.Revit.DB.Visual import AppearanceAssetEditScope

    found = [m for m in FilteredElementCollector(doc).OfClass(Material) if m.Name == spec["name"]]
    material = found[0] if found else doc.GetElement(Material.Create(doc, spec["name"]))
    r, g, b = spec["colour"]
    material.Color = Color(int(r), int(g), int(b))
    material.Shininess = int((1 - spec["roughness"]) * 128)
    if base is None:
        return material
    if not found or material.AppearanceAssetId.Equals(ElementId.InvalidElementId):
        same = [e for e in FilteredElementCollector(doc).OfClass(AppearanceAssetElement) if e.Name == spec["name"]]
        material.AppearanceAssetId = (same[0] if same else base.Duplicate(spec["name"])).Id
    scope = AppearanceAssetEditScope(doc)
    asset = scope.Start(material.AppearanceAssetId)
    asset.FindByName("generic_diffuse").SetValueAsColor(material.Color)
    gloss = asset.FindByName("generic_glossiness")
    if gloss is not None:
        gloss.Value = max(0.0, min(1.0, 1 - spec["roughness"]))
    if spec.get("image"):
        path = os.path.join(folder, "textures", spec["image"] + ".jpg")
        if os.path.exists(path):
            set_bitmap(asset, "generic_diffuse", path, spec)
            bump = os.path.join(folder, "textures", spec["image"] + "-h1.png")
            if os.path.exists(bump):
                try:
                    set_bitmap(asset, "generic_bump_map", bump, spec)
                    asset.FindByName("generic_bump_amount").Value = 0.1
                except Exception as e:
                    notes.append("%s: no relief (%s)" % (spec["name"], e))
        else:
            notes.append("%s: textures/%s.jpg is missing, so it is plain colour" % (spec["name"], spec["image"]))
    metal = asset.FindByName("generic_is_metal")
    if metal is not None:
        metal.Value = bool(spec.get("metal"))
    scope.Commit(True)
    return material


def generic_appearance(doc):
    """A Generic appearance to copy: Revit's other kinds have no slot for a picture."""
    from Autodesk.Revit.DB import AppearanceAssetElement, FilteredElementCollector
    from Autodesk.Revit.DB.Visual import AssetType

    for element in FilteredElementCollector(doc).OfClass(AppearanceAssetElement):
        if element.GetRenderingAsset().FindByName("generic_diffuse") is not None:
            return element
    for asset in doc.Application.GetAssets(AssetType.Appearance):
        if asset.FindByName("generic_diffuse") is not None:
            return AppearanceAssetElement.Create(doc, PREFIX + "Generic", asset)
    return None


def clear_last_import(doc, tag):
    """Running it again replaces this job's parts, and leaves everything else in the model."""
    from Autodesk.Revit.DB import BuiltInParameter, DirectShape, ElementId, FilteredElementCollector, Group
    from System.Collections.Generic import List

    for group in list(FilteredElementCollector(doc).OfClass(Group)):
        if group.GroupType.Name.startswith(tag):
            doc.Delete(group.Id)
    old = List[ElementId]()
    for shape in FilteredElementCollector(doc).OfClass(DirectShape):
        comment = shape.get_Parameter(BuiltInParameter.ALL_MODEL_INSTANCE_COMMENTS)
        if comment is not None and comment.AsString() == tag:
            old.Add(shape.Id)
    if old.Count:
        doc.Delete(old)


def make_shape(doc, body, verts, material_id, tag):
    from Autodesk.Revit.DB import (
        BuiltInCategory,
        BuiltInParameter,
        DirectShape,
        ElementId,
        GeometryObject,
        TessellatedFace,
        TessellatedShapeBuilder,
        TessellatedShapeBuilderFallback,
        TessellatedShapeBuilderTarget,
        XYZ,
    )
    from System.Collections.Generic import List

    points = {}

    def point(i):
        if i not in points:
            x, y, z = verts[i]
            points[i] = XYZ(x * FEET_PER_MM, y * FEET_PER_MM, z * FEET_PER_MM)
        return points[i]

    builder = TessellatedShapeBuilder()
    builder.OpenConnectedFaceSet(False)
    for a, b, c in body["triangles"]:
        loop = List[XYZ]()
        for i in (a, b, c):
            loop.Add(point(i))
        builder.AddFace(TessellatedFace(loop, material_id))
    builder.CloseConnectedFaceSet()
    builder.Target = TessellatedShapeBuilderTarget.AnyGeometry
    builder.Fallback = TessellatedShapeBuilderFallback.Mesh
    builder.Build()
    shape = DirectShape.CreateElement(doc, ElementId(BuiltInCategory.OST_Casework))
    objects = List[GeometryObject]()
    for g in builder.GetBuildResult().GetGeometricalObjects():
        objects.Add(g)
    shape.SetShape(objects)
    shape.Name = body["name"]
    shape.get_Parameter(BuiltInParameter.ALL_MODEL_INSTANCE_COMMENTS).Set(tag)
    mark = shape.get_Parameter(BuiltInParameter.ALL_MODEL_MARK)
    if mark is not None and not mark.IsReadOnly:
        mark.Set(body["name"])
    return shape


def job_folder():
    """The folder this graph was opened from, or one the user picks."""
    try:
        clr.AddReference("DynamoRevitDS")
        import Dynamo

        path = Dynamo.Applications.DynamoRevit.RevitDynamoModel.CurrentWorkspace.FileName
        if path and os.path.exists(os.path.join(os.path.dirname(path), "job.json")):
            return os.path.dirname(path)
    except Exception:
        pass
    clr.AddReference("System.Windows.Forms")
    from System.Windows.Forms import DialogResult, FolderBrowserDialog

    dialog = FolderBrowserDialog()
    dialog.Description = "The Cabinet Studio job's folder, with job.json in it"
    return dialog.SelectedPath if dialog.ShowDialog() == DialogResult.OK else None


def main():
    clr.AddReference("RevitAPI")
    clr.AddReference("RevitServices")
    from Autodesk.Revit.DB import ElementId
    from RevitServices.Persistence import DocumentManager
    from RevitServices.Transactions import TransactionManager
    from System.Collections.Generic import List

    doc = DocumentManager.Instance.CurrentDBDocument
    if doc.IsFamilyDocument:
        return "Open a project first: this imports into a project, not a family."
    folder = job_folder()
    if not folder:
        return "No folder chosen: nothing imported."
    with open(os.path.join(folder, "job.json"), encoding="utf-8") as f:
        job = json.load(f)
    verts, bodies = parse_obj(os.path.join(folder, "job.obj"))
    kd = parse_mtl(os.path.join(folder, "job.mtl"))
    parts = {p["name"]: p for p in job["parts"]}
    tag = "Cabinet Studio: " + os.path.basename(folder.rstrip("\\/"))
    notes, materials, by_cabinet, failed = [], {}, {}, []
    TransactionManager.Instance.EnsureInTransaction(doc)
    clear_last_import(doc, tag)
    base = generic_appearance(doc)
    if base is None:
        notes.append("No Generic appearance found: materials are plain colours.")
    for body in bodies:
        part = parts.get(body["name"])
        spec = material_of(body, part, job["boards"], kd)
        try:
            if spec["key"] not in materials:
                materials[spec["key"]] = make_material(doc, spec, folder, base, notes)
            shape = make_shape(doc, body, verts, materials[spec["key"]].Id, tag)
            by_cabinet.setdefault(body["name"].split(".")[0], []).append(shape.Id)
        except Exception as e:
            failed.append("%s: %s" % (body["name"], e))
    for cabinet, ids in by_cabinet.items():
        try:
            group = doc.Create.NewGroup(List[ElementId](ids))
            group.GroupType.Name = "%s %s" % (tag, cabinet)
        except Exception as e:
            notes.append("%s not grouped: %s" % (cabinet, e))
    TransactionManager.Instance.TransactionTaskDone()
    made = sum(len(v) for v in by_cabinet.values())
    report = ["%d of %d parts, %d materials, from %s" % (made, len(bodies), len(materials), folder)]
    return report + notes + ["FAILED " + x for x in failed]


if IN_REVIT:
    OUT = main()
