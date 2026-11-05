import Foundation

@main
struct ToolRunnerTests {
    static func main() throws {
        let shell = URL(fileURLWithPath: "/bin/sh")
        // A successful export must still open when a third-party add-on prints a traceback.
        try ToolRunner.run(shell, ["-c", "echo 'Traceback (most recent call last):'; echo 'scene: 94 bodies'; exit 0"])
        do {
            try ToolRunner.run(shell, ["-c", "echo 'scene could not be saved'; exit 7"])
            fatalError("A failed scene export was accepted")
        } catch let error as ToolRunner.Failure {
            precondition(error.description.contains("exit 7"))
            precondition(error.description.contains("scene could not be saved"))
        }
        print("Tool runner checks passed")
    }
}
