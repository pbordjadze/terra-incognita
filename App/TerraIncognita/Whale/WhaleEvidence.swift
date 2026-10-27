import SwiftUI
import UIKit

/// Offline paraphrases and a textual equivalent. Sources open only through an explicit link.
enum WhaleReading: String, Identifiable {
    case evidence, limbs, story
    var id: String { rawValue }
    var title: String {
        switch self {
        case .evidence: "The evidence"
        case .limbs: "The tiny hind limbs"
        case .story: "The visual story, in words"
        }
    }
}

struct WhaleEvidence: View {
    let reading: WhaleReading
    @Environment(\.dismiss) private var dismiss
    private let ink = Color(red: 0.17, green: 0.20, blue: 0.21)
    private let stone = Color(red: 0.94, green: 0.92, blue: 0.87)
    private let sea = Color(red: 0.20, green: 0.36, blue: 0.40)

    var body: some View {
        NavigationStack {
            ScrollView {
                VStack(alignment: .leading, spacing: 28) {
                    switch reading {
                    case .evidence: evidence
                    case .limbs: limbs
                    case .story: story
                    }
                }
                .font(.custom("LibertinusSerif-Regular", size: 18, relativeTo: .body))
                .foregroundStyle(ink)
                .frame(maxWidth: 620, alignment: .leading)
                .padding(.horizontal, 26)
                .padding(.top, 20)
                .padding(.bottom, 44)
                .frame(maxWidth: .infinity)
            }
            .accessibilityIdentifier("whaleReadingScroll")
            .background(stone)
            .navigationTitle(reading.title)
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .confirmationAction) {
                    Button { dismiss() } label: {
                        Text("Return").frame(minWidth: 44, minHeight: 44)
                    }
                    .accessibilityIdentifier("whaleCloseEvidence")
                }
            }
            .tint(sea)
        }
        .accessibilityIdentifier("whaleReading")
    }

    @ViewBuilder private var evidence: some View {
        passage("A sea in today's desert", "Wadi Al-Hitan is a whale-fossil locality in Egypt's Western Desert. Its whale-bearing rocks preserve ancient marine environments. The living Dorudon in this experience represents a fully aquatic whale that inhabited this region in the late Eocene, about 37 million years ago.")
        passage("A place, rather than one animal", "The illustrated fossil and the living whale connect the same kind of ancient animal with a real fossil-bearing place. They do not identify one individual, its death, or its exact excavation. The fossil's articulation, completeness and setting are illustrative reconstructions.")
        passage("The globe is a geographic reference", "Present-day coastlines and relief help locate inland Egypt. They are not an Eocene paleogeographic reconstruction. Modern political boundaries and settlements are hidden in the ancient scene. The local marine illustration's edge is a composition boundary, not a measured Tethys shoreline.")
        passage("Millions of years are omitted", "The cut from about 37 million years ago to today jumps across a long geological history. It does not show a single tide retreating, a whale becoming stranded, or an exact date when the modern desert formed.")
        ForEach(Self.sources) { source in sourceView(source) }
        passage("What the art reconstructs", "The illustrations were generated for this experiment. No museum artwork or fossil photograph is reproduced. Skin, color, soft-tissue outlines, pose, light, seabed, sand and movement are artistic reconstructions. A modest horizontal fluke is an anatomical inference; its exact outline is not preserved by the cited skeletons. These notes are a source-based implementation review, not professional paleontological approval of every rendered scene.", identifier: "whaleEvidenceReconstruction")
        passage("Available offline", "These summaries and the visual story in words are bundled with the atlas. The original papers and institutional pages are additional reading, opened only when you choose a source link.")
    }

    @ViewBuilder private var limbs: some View {
        passage("Fully aquatic, with small rear limbs", "Dorudon retained tiny hind limbs and feet, even though it lived fully in the sea. They could not support walking or provide swimming propulsion. Their exact function is not assigned here.", identifier: "whaleLimbClaim")
        VStack(alignment: .leading, spacing: 12) {
            heading("Where they belonged")
                .accessibilityIdentifier("whaleLimbsWholeBody")
            illustration(WhaleArt.image(.living), label: "Illustrated whole Dorudon: a long body and tail, front flippers, and very small hind limbs toward the rear.", height: 200)
            Text("The whole-body illustration puts the small limbs in context. The long torso and tail carried most of the animal's length; the rear limbs were much reduced.")
        }
        VStack(alignment: .leading, spacing: 12) {
            heading("Hind limb · enlarged illustration")
                .accessibilityIdentifier("whaleLimbsDetail")
            illustration(WhaleArt.image(.limb), label: "Enlarged illustrative detail of the rear flank and tiny hind limbs; displayed at a different scale from the whole animal.", height: 260)
                .accessibilityIdentifier("whaleLimbDetail")
            Text("This detail is enlarged independently of the whole animal. Bone shapes inform the reconstruction, while its skin covering and pose are inferred. It is an anatomical illustration, not a scan of a particular specimen.")
        }
        passage("How did it swim?", "Vertebral anatomy supports movement involving the body and tail. A 2019 study discusses mid-body-through-tail undulation in Dorudon and questions how much thrust its small inferred flukes supplied. The gentle animation is one restrained interpretation, not a measured gait or proof of an exclusively modern, tail-fluke-powered stroke. The tiny hind limbs remain passive.")
        sourceView(Self.sources[4])
        sourceView(Self.sources[5])
        sourceView(Self.sources[2])
    }

    @ViewBuilder private var story: some View {
        passage("Today · Wadi Al-Hitan, Egypt", "A small illustrated whale fossil lies in the atlas at Wadi Al-Hitan, in Egypt's Western Desert. It represents fossils found in this region, rather than one recorded skeleton at an exact position. Touching it offers a closer look and a voluntary visit to its past.")
        passage("Back to about 37 million years ago", "The camera widens to locate inland Egypt relative to the Mediterranean and Red Sea, then approaches the fossil region. The present-day fossil recedes. The date changes to about 37 million years ago.", identifier: "whaleStoryAncient")
        passage("A shallow sea", "An illustrated marine setting opens around the same geographic anchor. The surrounding globe remains a modern geographic reference. The marine illustration represents the ancient environment recorded by local rocks; its edge does not claim an exact ancient shoreline.")
        passage("Dorudon", "A living Dorudon reconstruction appears: a long, streamlined whale with a narrow snout, front flippers, a long tail and tiny hind limbs. Gentle movement passes through its body and tail. Its small rear limbs do not paddle. This whale was already fully aquatic, and is not portrayed as the first whale or a uniquely identified ancestor of every living whale.")
        passage("Tiny hind limbs", "The composition draws attention to the small limbs near the rear of the animal. They could not bear its weight on land or propel it through the water. You can inspect their place in the whole body and an enlarged illustrative detail, then return whenever you wish.")
        passage("Much later · Today", "A deliberate cut skips the intervening millions of years. The marine setting yields to the atlas and the present-day desert. This is a large jump through time, not one sea draining into dunes, a sudden desertification event, or the death of the illustrated whale.")
        passage("Whales swam here", "The camera settles again on Wadi Al-Hitan. The fossil remains in today's desert. The place now means something different: whales once swam here. You can inspect the evidence, revisit a moment, move away, or leave and continue exploring the atlas. There is no required ending.")
        passage("Reading and controlling the images", "The animals and habitats are illustrative reconstructions, and the ancient date is approximate. You can pause the sequence, choose its documented moments, or use the low-motion presentation. Moving the globe pauses the guide and yields the camera; resuming brings the authored view back from your current position.")
    }

    private func illustration(_ image: UIImage?, label: String, height: CGFloat) -> some View {
        Group {
            if let image {
                Image(uiImage: image)
                    .resizable()
                    .scaledToFit()
                    .frame(maxWidth: .infinity)
                    .frame(height: height)
                    .accessibilityLabel(label)
            } else {
                Text(label)
                    .font(.custom("LibertinusSerif-Regular", size: 17, relativeTo: .body))
            }
        }
    }

    private func heading(_ title: String) -> some View {
        Text(title)
            .font(.custom("LibertinusSerif-Semibold", size: 24, relativeTo: .title2))
            .accessibilityAddTraits(.isHeader)
    }

    private func passage(_ title: String, _ body: String, identifier: String? = nil) -> some View {
        VStack(alignment: .leading, spacing: 9) {
            heading(title)
            Text(body).fixedSize(horizontal: false, vertical: true)
                .accessibilityIdentifier(identifier ?? title)
        }
    }

    private func sourceView(_ source: Source) -> some View {
        VStack(alignment: .leading, spacing: 8) {
            heading(source.title)
            Text(source.detail)
                .font(.custom("LibertinusSerif-Regular", size: 17, relativeTo: .body))
            Text(source.support).fixedSize(horizontal: false, vertical: true)
            if let url = URL(string: source.url) {
                Link("Open source ↗", destination: url)
                    .padding(.vertical, 10)
                    .accessibilityLabel("Open source: \(source.title)")
            }
        }
    }

    private struct Source: Identifiable {
        var id: String { url }
        let title: String
        let detail: String
        let support: String
        let url: String
    }

    private static let sources: [Source] = [
        Source(title: "Wadi Al-Hitan", detail: "UNESCO World Heritage Centre · property 1186", support: "The institutional listing identifies this whale-fossil locality in Egypt's Western Desert. The atlas uses the property's reference point to locate the region, not as excavation coordinates for the illustrated skeleton.", url: "https://whc.unesco.org/en/list/1186"),
        Source(title: "The rocks and their whales", detail: "Peters, Antar, Zalmout & Gingerich · PALAIOS · 2009", support: "The abstract and geological context document articulated Dorudon skeletons and marine shelf-to-shoreface deposits at Wadi Al-Hitan. This supports an ancient local sea. It does not establish a single date or cause for the modern Sahara's formation.", url: "https://strata.geology.wisc.edu/reprints/Peters_etal2009.pdf"),
        Source(title: "Dorudon in the late Eocene", detail: "Philip Gingerich · Evolution of whales from land to sea · 2012", support: "Pages 315–317 and Figure 7 informed the approximate 37-million-year scene date and long-bodied, reduced-limb anatomy. The life reconstruction in that paper is copyrighted reference artwork and is not reused here. Soft-tissue shape and swimming mechanics remain reconstructed.", url: "https://lsa.umich.edu/content/dam/paleontology-assets/paleontology-documents/1560305Gingerich.pdf"),
        Source(title: "Fossil whales, described", detail: "Smithsonian National Museum of Natural History · Sant Ocean Hall", support: "The accessible museum description places Dorudon about 38–36 million years ago and describes its reduced rear legs and aquatic life. That range supports the rounded scene date. Its simplified swimming account is considered alongside the later locomotor study below.", url: "https://naturalhistory.si.edu/visit/accessibility/audio-and-visual-description/sant-ocean-hall-visual-description-highlights-tour"),
        Source(title: "Limbs that no longer propelled", detail: "Gavazzi and colleagues · Journal of Vertebrate Paleontology · 2020", support: "The comparative discussion on page 3 identifies dramatically reduced hind limbs as nonfunctional in locomotion. The rear limbs in this experience never walk, bear the animal's weight or paddle. Their exact nonlocomotor function is left open.", url: "https://cooperbonelab.com/wp-content/uploads/2020/12/gavazzietal2020archaeocetecarpals-1.pdf"),
        Source(title: "Body, tail and inferred flukes", detail: "Gingerich, Antar & Zalmout · PLOS ONE · 2019; corrected 2020", support: "The Dorudon-specific discussion and Figures 25–26 consider mid-body-through-tail undulation and the uncertain contribution of its small inferred flukes. The animation does not assert an exclusively modern-dolphin swimming mechanism. The corrected publisher text was consulted.", url: "https://journals.plos.org/plosone/article?id=10.1371/journal.pone.0225391")
    ]
}
