import Foundation
import simd

@main struct StoneGeometryTests {
    static func main() {
        func check(_ cuts: [Double], _ expected: Double) {
            let pieces = StoneGeometry.pieces(min: [0,0,0], max: [100,20,60], cuts: cuts)
            let volume = pieces.map(StoneGeometry.volume).reduce(0,+)
            precondition(abs(volume-expected) < 0.01, "Volume \(volume), expected \(expected)")
            for solid in pieces {
                // A closed oriented surface has zero summed area vector.
                var area = SIMD3<Double>.zero
                for f in solid { for i in 1..<(f.count-1) { area += simd_cross(f[i]-f[0],f[i+1]-f[0])/2 } }
                precondition(simd_length(area) < 0.001, "Open mitred mesh")
            }
        }
        check([],120000)
        // Upper inside arris, through the full 100 mm length.
        check([0,0,40, 100,20,60, 50,20,60],100000)
        // A front mitre on the last 40 mm of a waterfall: do not cut its full height.
        check([80,0,20, 100,20,60, 100,0,40],112000)
        check([0,0,20, 20,20,60, 20,0,40],112000)
        check([0,0,40, 100,20,60, 50,20,60, 0,0,0, 20,20,60, 0,20,30],90666.66666666667)
        let notched = StoneGeometry.pieces(min: [0,0,0], max: [100,30,18], cuts: [], voids: [0,0,15,100,10,18])
        precondition(abs(notched.map(StoneGeometry.volume).reduce(0,+)-51000) < 0.01, "Shadow rail clearance notch missing")
        // Lower seating skin: upper outside mitres on both ends and the rear return.
        let skin = StoneGeometry.pieces(min: [0,0,0], max: [100,300,20], cuts: [
            0,0,0,20,300,20,0,150,20, 80,0,0,100,300,20,100,150,20,
            0,280,0,100,300,20,50,300,20])
        precondition(abs(skin.map(StoneGeometry.volume).reduce(0,+)-465333.3333333333)<0.01, "Lower skin mitres overlap")
        for solid in skin {
            var area = SIMD3<Double>.zero
            for f in solid { for i in 1..<(f.count-1) { area += simd_cross(f[i]-f[0],f[i+1]-f[0])/2 } }
            precondition(simd_length(area)<0.001, "Open lower skin mesh")
        }
        print("Stone preview geometry passed")
    }
}
