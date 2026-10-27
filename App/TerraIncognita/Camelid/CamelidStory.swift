import Foundation
import Observation
import UIKit

/// This clock belongs only to the authored camelid sequence. It is not a global historical engine.
@Observable
final class CamelidStory {
    static let duration = 87.0
    static let milestones = [0.0, 10, 26, 39, 54, 69]
    var followed = false
    var playing = false
    var exploring = false
    @ObservationIgnored var heldComposition = false
    var scene = 0
    var date = "Today"
    var elapsed = 0.0
    var reading: CamelidReading?
    @ObservationIgnored weak var canvas: CamelidCanvas?
    @ObservationIgnored var preciseTime = 0.0

    init() {
        #if DEBUG
        let args = ProcessInfo.processInfo.arguments
        if let i = args.firstIndex(of: "-camelid"), i + 1 < args.count, let t = Double(args[i + 1]) {
            followed = true
            heldComposition = true
            preciseTime = min(max(t, 0), Self.duration)
        } else if args.contains("-camelid-play") {
            followed = true
            playing = true
        }
        #endif
        reportTime()
    }

    var title: String {
        if exploring { return "" }
        guard followed else { return "Dromedary camel" }
        return ["", "Where camelids began", "One family · two lineages", "Toward Eurasia", "Toward South America", "Camels and llamas are cousins."][scene]
    }
    var place: String {
        if exploring { return "" }
        guard followed else { return "Arabia" }
        return ["", "North America", "North America", "Beringia", "The Americas", ""][scene]
    }
    var historical: Bool { followed && scene < 5 }
    var description: String {
        guard followed else { return "A living dromedary camel in Arabia. Follow its lineage or return to the atlas." }
        return [
            "We trace the camel's ancestry backward through time, toward North America. Different illustrations represent different periods; the modern camel does not turn into an ancient animal.",
            "About 44 million years ago, the camelid family originated in North America. Early members were small and humpless. This animal is an illustrative reconstruction, not a complete fossil specimen.",
            "By about 17 million years ago, the lineages leading to Old World camels and South American camelids had separated, while still in North America. This branching diagram shows family relationships, not travel.",
            "Around 7 million years ago, a camelid lineage dispersed through the Beringian region into Eurasia. The broad band represents population expansion over many generations, not one animal's route.",
            "Around 3 million years ago, another camelid lineage dispersed south through the Americas. These were wild ancestral animals, not domestic llamas or alpacas.",
            "Today, camels and llamas are cousins. Their shared family began in North America; ancestral branches spread into Eurasia and South America. Llamas and alpacas are domestic camelids. The globe is yours to explore."
        ][scene]
    }

    func follow() { exploring = false; heldComposition = false; followed = true; preciseTime = 0; reportTime(); canvas?.seek(); playing = true }
    func inspectCamel() { exploring = false; followed = false; playing = false; preciseTime = 0; reportTime(); canvas?.seek() }
    func pause() { playing = false; reportTime() }
    func resume() {
        exploring = false
        heldComposition = false
        if preciseTime >= Self.duration { preciseTime = 69; reportTime() }
        canvas?.resumeFromCurrentCamera()
        playing = true
    }
    func select(_ index: Int) {
        guard Self.milestones.indices.contains(index) else { return }
        exploring = false; heldComposition = true; followed = true; playing = false; preciseTime = Self.milestones[index]
        reportTime(); canvas?.seek()
    }
    func open(_ choice: CamelidReading) { pause(); reading = choice }
    func reportTime() {
        let fresh = Self.milestones.lastIndex(where: { preciseTime >= $0 }) ?? 0
        if scene != fresh { scene = fresh }
        let nextDate: String
        if !followed || preciseTime == 0 { nextDate = "Today" }
        else if fresh == 0 { nextDate = "≈\(Int((44 * min(preciseTime / 10, 1)).rounded())) Ma" }
        else { nextDate = ["", "≈44 Ma", "≈17 Ma", "≈7 Ma", "≈3 Ma", "Today"][fresh] }
        if nextDate != date { date = nextDate }
        // The accessible transport value changes at a reading pace, never 60 times per second.
        if abs(elapsed - preciseTime) >= 0.5 || !playing { elapsed = preciseTime }
    }
}
