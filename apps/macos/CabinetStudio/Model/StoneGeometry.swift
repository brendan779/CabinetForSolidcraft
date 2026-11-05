import Foundation
import simd

/// Convex pieces of a stone blank after finite triangular mitre cuts, in cabinet millimetres.
/// Keeping the cuts finite matters where a front return meets only the top of a waterfall.
enum StoneGeometry {
    typealias Point = SIMD3<Double>
    typealias Face = [Point]
    typealias Solid = [Face]

    static func pieces(min lo: Point, max hi: Point, cuts: [Double], voids: [Double] = []) -> [Solid] {
        func p(_ x: Int, _ y: Int, _ z: Int) -> Point { [x == 0 ? lo.x : hi.x, y == 0 ? lo.y : hi.y, z == 0 ? lo.z : hi.z] }
        var solids: [Solid] = [[
            [p(0,0,0),p(0,1,0),p(1,1,0),p(1,0,0)],
            [p(0,0,1),p(1,0,1),p(1,1,1),p(0,1,1)],
            [p(0,0,0),p(1,0,0),p(1,0,1),p(0,0,1)],
            [p(0,1,0),p(0,1,1),p(1,1,1),p(1,1,0)],
            [p(0,0,0),p(0,0,1),p(0,1,1),p(0,1,0)],
            [p(1,0,0),p(1,1,0),p(1,1,1),p(1,0,1)]
        ]]
        if voids.count >= 6 {
            for i in stride(from: 0, through: voids.count-6, by: 6) {
                let l = Point(voids[i],voids[i+1],voids[i+2]), h = Point(voids[i+3],voids[i+4],voids[i+5])
                solids = solids.flatMap { solid -> [Solid] in
                    var outside: [Solid] = [], middle: Solid? = solid
                    for j in 0..<3 {
                        guard let current = middle else { break }
                        var axis = Point.zero; axis[j] = 1
                        if let below = clip(current, normal: -axis, distance: -l[j]) { outside.append(below) }
                        if let above = clip(current, normal: axis, distance: h[j]) { outside.append(above) }
                        middle = clip(current, normal: axis, distance: l[j]).flatMap { clip($0, normal: -axis, distance: -h[j]) }
                    }
                    return outside
                }
            }
        }
        guard cuts.count >= 9 else { return solids }
        for i in stride(from: 0, through: cuts.count - 9, by: 9) {
            let l = Point(cuts[i], cuts[i+1], cuts[i+2])
            let h = Point(cuts[i+3], cuts[i+4], cuts[i+5])
            let at = Point(cuts[i+6], cuts[i+7], cuts[i+8])
            guard let a = (0..<3).first(where: { at[$0] > l[$0]+1e-6 && at[$0] < h[$0]-1e-6 }) else { continue }
            let cross = (0..<3).filter { $0 != a }
            guard cross.allSatisfy({ h[$0] > l[$0] }) else { continue }
            var n = Point.zero
            for j in cross { n[j] = (abs(at[j]-l[j]) < 1e-6 ? 1 : -1)/(h[j]-l[j]) }
            let d = simd_dot(n, at) + 1
            solids = solids.flatMap { solid -> [Solid] in
                var result: [Solid] = []
                var middle: Solid? = solid
                // Preserve material outside every side of the finite cutting prism.
                // A return's mitre may end inside the adjoining horizontal slab.
                for j in 0..<3 {
                    guard let current = middle else { break }
                    var axis = Point.zero; axis[j] = 1
                    if let below = clip(current, normal: -axis, distance: -l[j]) { result.append(below) }
                    if let above = clip(current, normal: axis, distance: h[j]) { result.append(above) }
                    middle = clip(current, normal: axis, distance: l[j]).flatMap { clip($0, normal: -axis, distance: -h[j]) }
                }
                if let middle, let kept = clip(middle, normal: n, distance: d) { result.append(kept) }
                return result
            }
        }
        return solids
    }

    /// Keep dot(normal, point) >= distance and close the new face.
    private static func clip(_ solid: Solid, normal: Point, distance: Double) -> Solid? {
        let eps = 1e-7
        let distances = solid.flatMap { $0 }.map { simd_dot(normal,$0)-distance }
        if distances.allSatisfy({ $0 >= -eps }) { return solid }
        if distances.allSatisfy({ $0 <= eps }) { return nil }
        var faces: Solid = [], rim: Face = []
        for face in solid {
            var out: Face = []
            for i in face.indices {
                let a = face[i], b = face[(i+1)%face.count]
                let da = simd_dot(normal,a)-distance, db = simd_dot(normal,b)-distance
                if da >= -eps { out.append(a) }
                if (da > eps && db < -eps) || (da < -eps && db > eps) {
                    let p = a+(b-a)*(da/(da-db))
                    out.append(p); rim.append(p)
                } else if abs(da) <= eps { rim.append(a) }
            }
            var unique: Face = []
            for p in out where unique.last.map({ simd_distance($0,p) > eps }) ?? true { unique.append(p) }
            if unique.count > 1, simd_distance(unique[0],unique.last!) < eps { unique.removeLast() }
            if unique.count >= 3 { faces.append(unique) }
        }
        var ring: Face = []
        for p in rim where !ring.contains(where: { simd_distance($0,p) < eps }) { ring.append(p) }
        if ring.count >= 3 {
            let centre = ring.reduce(.zero,+)/Double(ring.count)
            let n = simd_normalize(normal)
            let ref = abs(n.x) < 0.8 ? Point(1,0,0) : Point(0,1,0)
            let u = simd_normalize(simd_cross(n,ref)), v = simd_cross(n,u)
            ring.sort { atan2(simd_dot($0-centre,v),simd_dot($0-centre,u)) < atan2(simd_dot($1-centre,v),simd_dot($1-centre,u)) }
            // This cap faces toward the discarded half space.
            faces.append(ring.reversed())
        }
        guard volume(faces) > eps else { return nil }
        return faces
    }

    static func volume(_ solid: Solid) -> Double {
        var volume = 0.0
        for f in solid where f.count >= 3 {
            for i in 1..<(f.count-1) { volume += simd_dot(f[0],simd_cross(f[i],f[i+1]))/6 }
        }
        return abs(volume)
    }
}
