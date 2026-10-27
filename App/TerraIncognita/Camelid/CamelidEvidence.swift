import SwiftUI

/// Bundled paraphrases and accessible equivalents; opening a source is always a reader action.
enum CamelidReading: String, Identifiable {
    case evidence, family, story
    var id: String { rawValue }
    var title: String {
        switch self {
        case .evidence: "The evidence"
        case .family: "The South American family"
        case .story: "The visual story, in words"
        }
    }
}

struct CamelidEvidence: View {
    var reading: CamelidReading = .evidence
    @Environment(\.dismiss) private var dismiss
    private let ink = Color(red: 0.23, green: 0.17, blue: 0.09)
    private let paper = Color(red: 0.95, green: 0.89, blue: 0.78)
    private let blue = Color(red: 0.31, green: 0.30, blue: 0.47)

    var body: some View {
        NavigationStack {
            ScrollView {
                VStack(alignment: .leading, spacing: 26) {
                    switch reading {
                    case .evidence: evidence
                    case .family: family
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
            .accessibilityIdentifier("camelidReadingScroll")
            .background(paper)
            .navigationTitle(reading.title)
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .confirmationAction) {
                    Button("Return") { dismiss() }
                        .accessibilityIdentifier("camelidCloseEvidence")
                }
            }
            .tint(blue)
        }
        .accessibilityIdentifier("camelidReading")
    }

    @ViewBuilder private var evidence: some View {
        passage("A family, across deep time", "Camels and llamas share ancestry. Their family originated in North America. The lineages leading to living Old World and South American camelids separated there long before their ancestors dispersed to other continents.")
        passage("Dates are approximate", "The scene dates summarize broad evolutionary events. About 17 million years ago is a published estimate of lineage divergence, not a fossil-observed moment when one animal became two species. Dispersal washes represent populations spreading over generations; their edges are illustrative, not measured routes.")
        passage("The map is a reference", "The globe retains present-day coastlines and relief to help you recognize the regions. It is not a paleogeographic reconstruction. Modern countries and settlements are hidden in prehistoric scenes. Beringia is identified as a regional connection, without claiming its exact shoreline at a particular year.")
        passage("An ancient animal is an illustration", "The early camelid is a representative, humpless form informed by the middle-Eocene genus Poebrodon. Its fragmentary jaw and tooth fossils do not establish a complete life appearance. Body proportions, coat, color, pose and scenery are artistic reconstructions. The later animals represent a Paracamelus-like northern lineage and a Hemiauchenia-like southern lineage; neither is a uniquely identified migrant or a proven direct ancestor.")
        ForEach(Self.sources) { source in
            sourceView(source)
        }
        passage("Artwork & scope", "The animal artwork was generated for this experiment and composed within the atlas. No museum illustration is reproduced. These notes record a source-based implementation review, not professional paleontological approval of every rendered scene. The source articles themselves are not bundled; these summaries remain available offline.")
    }

    @ViewBuilder private var family: some View {
        passage("Two domestic Andean relatives", "Llamas and alpacas belong to the South American camelid branch. Both are domesticated animals, so they appear only after the sequence returns to today.")
        VStack(alignment: .leading, spacing: 12) {
            CamelidAnimalView(kind: .llama)
                .frame(height: 220)
                .frame(maxWidth: .infinity)
                .accessibilityHidden(true)
            heading("Llama")
                .accessibilityIdentifier("camelidFamilyLlama")
            Text("The larger, longer-faced animal. Its longer neck and more open body silhouette distinguish it from the compact, fleece-covered alpaca.")
        }
        VStack(alignment: .leading, spacing: 12) {
            CamelidAnimalView(kind: .alpaca)
                .frame(height: 220)
                .frame(maxWidth: .infinity)
                .accessibilityHidden(true)
            heading("Alpaca")
                .accessibilityIdentifier("camelidFamilyAlpaca")
            Text("Smaller and more compact, with a dense fleece. Andean people domesticated alpacas for their wool. Modern coats vary; the illustrated animal represents one appearance, not every breed.")
        }
        passage("Their wild relatives", "Genetic research links llama domestication chiefly with guanacos, and alpaca domestication chiefly with vicuñas. Domestication and later interbreeding make this history more complex than four unchanging types. Living wild animals are relatives, not unchanged snapshots of prehistoric ancestors.")
        sourceView(Self.sources[4])
        sourceView(Self.sources[5])
        sourceView(Self.sources[6])
    }

    @ViewBuilder private var story: some View {
        passage("Today · Arabia", "A one-humped dromedary stands in Arabia. You can inspect it, follow its family history, or continue exploring the atlas.")
        passage("About 44 million years ago · North America", "The atlas moves backward through time and turns toward North America. The modern camel fades away. A much smaller, humpless early camelid is shown as an illustrative reconstruction. This is the continent where the camelid family began.")
        passage("About 17 million years ago · North America", "A branching family tree appears. Separate lineages lead toward Old World camels and South American camelids. Their separation predates both intercontinental dispersals. No single animal splits into two, and modern camels do not evolve directly into modern llamas.")
        passage("About 7 million years ago · Beringia", "The globe turns toward Alaska and northeast Asia. A broad wash shows a camelid lineage spreading from North America into Eurasia through the Beringian region. This represents populations over many generations, not one animal's journey or an exact fossil route.")
        passage("About 3 million years ago · The Americas", "The globe returns to the Americas. A second broad wash spreads south through Central America into South America. The animal represents an ancient lineage, not a domestic llama or alpaca. The wider Great American Biotic Interchange unfolded in stages.")
        passage("Today · The cousins", "The atlas returns to the present. A camel and a llama face one another, linked through their shared North American ancestry. Camels and llamas are cousins. Their family began in North America, and their ancestors spread in different directions. You can pause, revisit a moment, inspect the relatives, or leave and wander.")
        passage("Reading the images", "Present-day geography is retained as a familiar reference in ancient scenes; it does not claim to reconstruct ancient shorelines. The tree means evolutionary relationship. The broad colored washes mean approximate population dispersal. The animals are representative illustrations rather than observed individuals.")
    }

    private func heading(_ title: String) -> some View {
        Text(title)
            .font(.custom("LibertinusSerif-Semibold", size: 24, relativeTo: .title2))
            .accessibilityAddTraits(.isHeader)
    }

    private func passage(_ title: String, _ body: String) -> some View {
        VStack(alignment: .leading, spacing: 9) {
            heading(title)
            Text(body).fixedSize(horizontal: false, vertical: true)
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
                    .padding(.vertical, 9)
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
        Source(title: "The Camels", detail: "National Park Service · 2021", support: "The family originated in North America about 44 million years ago. The article places the northern dispersal at about 7–6 million years ago, and Hemiauchenia's southward dispersal at about 3–2 million years ago. These broad intervals inform the rounded scene dates.", url: "https://www.nps.gov/articles/000/the-camels.htm"),
        Source(title: "Camel evolution in the High Arctic", detail: "Rybczynski and colleagues · Nature Communications · 2013", support: "The introduction places the two living camelid branches' divergence by about 17 million years ago, and records Eurasian Paracamelus fossils about 7.5–6.5 million years old. Its Arctic camel fossil is younger than the northern dispersal scene; that individual is not portrayed as the migrant.", url: "https://pmc.ncbi.nlm.nih.gov/articles/PMC3615376/"),
        Source(title: "A branching fossil history", detail: "Donald Prothero · Evolution: Education and Outreach · 2009", support: "The camel section distinguishes early Poebrodon from later Poebrotherium, and emphasizes a branching evolutionary history. It treats Protylopus as an oromerycid, so that animal is not used here as an established early camelid.", url: "https://link.springer.com/article/10.1007/s12052-009-0136-1"),
        Source(title: "A prolonged interchange", detail: "Tseng and colleagues · Science · 2026", support: "The abstract describes the Great American Biotic Interchange as a prolonged, staged history. This supplies context for the southern dispersal without assigning a new exact camelid arrival date. The abstract and publication record were checked; the full paper and supplement were not available for this implementation review.", url: "https://pubmed.ncbi.nlm.nih.gov/42531382/"),
        Source(title: "Alpaca", detail: "Smithsonian National Zoo · animal account", support: "Used for the alpaca's compact, fleece-covered appearance and its domestication by Andean people. Its family-origin sentence conflicts with the paleontological sources and is not used for the experience's dates.", url: "https://nationalzoo.si.edu/animals/alpaca"),
        Source(title: "The wild ancestors", detail: "Kadwell and colleagues · Proceedings of the Royal Society B · 2001", support: "Genetic analysis supports guanaco–llama and vicuña–alpaca domestication relationships. The bibliographic record and abstract informed this brief comparison; detailed interbreeding histories are outside its scope.", url: "https://doi.org/10.1098/rspb.2001.1774"),
        Source(title: "Llama anatomy", detail: "Animal Diversity Web · Lama glama", support: "Used for coarse contemporary anatomy: a larger, longer-faced llama silhouette distinct from the alpaca. This account is not used for the deep-time dates or current phylogenetic relationships.", url: "https://animaldiversity.org/accounts/Lama_glama/")
    ]
}
