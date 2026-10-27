import Foundation
import Observation
import UIKit

/// Playback state for one authored visit to Wadi Al-Hitan, rather than a global historical clock.
@Observable
final class WhaleStory {
    static let duration = 82.0
    static let milestones = [0.0, 10, 25, 43, 56, 68]
    var followed = false
    var playing = false
    var exploring = false
    var scene = 0
    var date = "Today"
    var elapsed = 0.0
    var reading: WhaleReading?
    @ObservationIgnored var heldComposition = false
    @ObservationIgnored weak var canvas: WhaleCanvas?
    @ObservationIgnored var preciseTime = 0.0

    init() {
        #if DEBUG
        let arguments = ProcessInfo.processInfo.arguments
        if let index = arguments.firstIndex(of: "-whale"), index + 1 < arguments.count,
           let time = Double(arguments[index + 1]), time.isFinite {
            followed = true
            heldComposition = true
            preciseTime = min(max(time, 0), Self.duration)
        } else if arguments.contains("-whale-play") {
            followed = true
            playing = true
        }
        #endif
        reportTime()
    }

    static func scene(at time: Double) -> Int { milestones.lastIndex(where: { time >= $0 }) ?? 0 }
    var historical: Bool { followed && scene < 5 }
    var title: String {
        guard !exploring else { return "" }
        guard followed else { return "Whale fossil" }
        return ["Whale fossil", "A shallow sea", "Dorudon", "Tiny hind limbs", "", "Whales swam here."][scene]
    }
    var place: String {
        guard !exploring else { return "" }
        return scene == 4 && followed ? "" : "Wadi Al-Hitan · Egypt"
    }
    var description: String {
        guard followed else { return "A whale fossil lies at Wadi Al-Hitan in Egypt's Western Desert. Inspect it, choose to visit its past, or continue exploring the atlas." }
        if exploring { return "Freely exploring the globe. The guided visit is paused at \(date). Resume to return to its composition." }
        return [
            "The atlas locates Wadi Al-Hitan inland in present-day Egypt, then visits its past. The date changes to about 37 million years ago. This is a dated transition, not a simulation of every intervening year.",
            "About 37 million years ago, this place was part of a shallow marine environment. An illustrated marine view opens around the fossil locality. Modern geography remains a reference; the illustrated view does not trace a measured ancient shoreline.",
            "Dorudon, an extinct whale, swims through the reconstructed marine setting. Its body and tail undulate gently. Its living appearance and exact movement are artistic reconstructions informed by fossils and research.",
            "Dorudon retained tiny hind limbs. They could not support walking on land and are not shown paddling. Inspect the whole body and the small limb detail for the anatomical evidence and its limits.",
            "Much later, the scene returns to the present through a deliberate temporal cut. The sea yields to fossil and desert illustrations. This does not claim a precise date or a continuously observed process of desertification.",
            "Today, whale fossils remain at Wadi Al-Hitan in Egypt's desert. Whales swam here. You may inspect the fossil, revisit the ancient sea, look at the tiny hind limbs, or leave and wander."
        ][scene]
    }

    func follow() {
        exploring = false; heldComposition = false; followed = true; preciseTime = 0
        reportTime(); canvas?.seek(); playing = true
    }
    func inspectFossil() {
        exploring = false; heldComposition = false; followed = false; playing = false; preciseTime = 0
        reportTime(); canvas?.seek()
    }
    func pause() { playing = false; reportTime() }
    func resume() {
        exploring = false; heldComposition = false
        if preciseTime >= Self.duration { preciseTime = Self.milestones[5]; reportTime() }
        canvas?.resumeFromCurrentCamera()
        playing = true
    }
    func select(_ index: Int) {
        guard Self.milestones.indices.contains(index) else { return }
        exploring = false; heldComposition = true; followed = true; playing = false
        preciseTime = Self.milestones[index]
        reportTime(); canvas?.seek()
    }
    func open(_ choice: WhaleReading) { pause(); reading = choice }

    /// Safe to call each native tick: scene/date publish only at actual boundaries, and elapsed
    /// publishes at a reading pace. The renderer should use this same `scene` after calling it.
    func reportTime() {
        let fresh = Self.scene(at: preciseTime)
        if scene != fresh { scene = fresh }
        let freshDate: String
        if !followed || (fresh == 0 && preciseTime < 2) { freshDate = "Today" }
        else if fresh < 4 { freshDate = "≈37 Ma" }
        else if fresh == 4 { freshDate = "Much later" }
        else { freshDate = "Today" }
        if date != freshDate { date = freshDate }
        if abs(elapsed - preciseTime) >= 0.5 || !playing { elapsed = preciseTime }
    }
}
