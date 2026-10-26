import SwiftUI

/// One authored, entirely bundled encounter. Session state belongs to the reader, not a profile.
struct OceanExperience: View {
    @Environment(\.dismiss) private var dismiss
    @Environment(\.accessibilityReduceMotion) private var reduceMotion
    @State private var lens = "seeds"
    @State private var time = 0.0
    @State private var evidence = false
    @State private var place: String?
    @State private var borrowed = false

    static let paper = Color(red: 0.949, green: 0.894, blue: 0.784)
    static let ink = Color(red: 0.231, green: 0.173, blue: 0.086)
    static let blue = Color(red: 0.29, green: 0.30, blue: 0.49)

    var body: some View {
        VStack(spacing: 0) {
            HStack {
                Button { dismiss() } label: { Label("Atlas", systemImage: "arrow.left").frame(minHeight: 44) }
                    .accessibilityIdentifier("closeExperience")
                Spacer()
                Button { evidence = true } label: { Label("Evidence", systemImage: "text.book.closed").frame(minHeight: 44) }
                    .accessibilityIdentifier("evidenceButton")
            }
            .font(.custom("LibertinusSerif-Regular", size: 18, relativeTo: .body))
            .padding(.horizontal, 20)
            .frame(minHeight: 52)
            .background(Self.paper)
            Rectangle().fill(Self.blue.opacity(0.5)).frame(height: 0.7)
            ScrollViewReader { proxy in
              ScrollView {
                VStack(alignment: .leading, spacing: 22) {
                    heading.id("encounterTop")
                    lensMenu
                    OceanMap(lens: lens, time: time, onPlace: { place = $0 })
                        .frame(height: 320)
                    Text("Move the glass across the islands. Tap a place to stay with it.")
                        .font(.custom("LibertinusSerif-Italic", size: 17, relativeTo: .body))
                        .foregroundStyle(Self.blue)
                    if let place { placeNote(place).id("placeNote") }
                    Group {
                        switch lens {
                        case "words": words
                        case "people": lives
                        default: seeds
                        }
                    }
                    .transition(.opacity)
                    Rectangle().fill(Self.blue.opacity(0.4)).frame(height: 0.7)
                    Text("No itinerary to finish. Follow another trace, or return to the world.")
                        .font(.custom("LibertinusSerif-Italic", size: 17, relativeTo: .body))
                        .foregroundStyle(Self.blue)
                        .padding(.bottom, 24)
                }
                .padding(.horizontal, 22)
                .padding(.top, 26)
                .frame(maxWidth: 680)
                .frame(maxWidth: .infinity)
            }
            .accessibilityIdentifier("oceanExperience")
            .onChange(of: lens) { _, _ in
                withAnimation(reduceMotion ? nil : .easeInOut(duration: 0.3)) { proxy.scrollTo("encounterTop", anchor: .top) }
            }
            .onChange(of: place) { _, value in
                if value != nil {
                    withAnimation(reduceMotion ? nil : .easeInOut(duration: 0.3)) { proxy.scrollTo("placeNote", anchor: .top) }
                }
            }
            }
        }
        .foregroundStyle(Self.ink)
        .background(Self.paper.ignoresSafeArea())
        .preferredColorScheme(.light)
        .sheet(isPresented: $evidence) { OceanEvidenceView() }
        .onAppear { GlobeData.registerFonts() }
    }

    private var heading: some View {
        VStack(alignment: .leading, spacing: 8) {
            Text("MADAGASCAR  /  INDIAN OCEAN")
                .font(.custom("LibertinusSerif-Regular", size: 13, relativeTo: .caption))
                .tracking(1.5).foregroundStyle(Self.blue)
            Text(lens == "words" ? "An island with an echo" : lens == "people" ? "No ocean is empty" : "An ocean in a seed")
                .font(.custom("LibertinusSerif-Regular", size: 37, relativeTo: .largeTitle))
                .fixedSize(horizontal: false, vertical: true)
                .accessibilityAddTraits(.isHeader)
            Text(lens == "words" ? "The nearest shore is not always the nearest relation." : lens == "people" ? "The people are larger than the pattern." : "A grain can outlast the meal it belonged to.")
                .font(.custom("LibertinusSerif-Italic", size: 20, relativeTo: .title3))
                .fixedSize(horizontal: false, vertical: true)
        }
    }

    private var lensMenu: some View {
        HStack(spacing: 0) {
            lensButton("seeds", title: "Seeds", identifier: "lensSeeds")
            lensButton("words", title: "Words", identifier: "lensWords")
            lensButton("people", title: "Lives", identifier: "lensPeople")
        }
        .overlay(alignment: .bottom) { Rectangle().fill(Self.blue.opacity(0.35)).frame(height: 0.7) }
    }

    private func lensButton(_ key: String, title: String, identifier: String) -> some View {
        Button {
            withAnimation(reduceMotion ? nil : .easeInOut(duration: 0.4)) {
                lens = key
                place = nil
            }
        } label: {
            Text(title)
                .font(.custom("LibertinusSerif-Regular", size: 20, relativeTo: .title3))
                .frame(maxWidth: .infinity, minHeight: 48)
                .background(lens == key ? Self.blue.opacity(0.10) : .clear)
                .overlay(alignment: .bottom) {
                    Rectangle().fill(lens == key ? Self.blue : .clear).frame(height: 2)
                }
        }
        .buttonStyle(.plain)
        .accessibilityIdentifier(identifier)
        .accessibilityAddTraits(lens == key ? .isSelected : [])
    }

    private var seeds: some View {
        VStack(alignment: .leading, spacing: 18) {
            HStack(alignment: .center, spacing: 14) {
                SeedStudy(progress: time).frame(width: 90, height: 138).accessibilityHidden(true)
                VStack(alignment: .leading, spacing: 8) {
                    Text(time < 0.5 ? "Draw back the present" : "Early crop traces")
                        .font(.custom("LibertinusSerif-Regular", size: 26, relativeTo: .title2))
                    Text(time < 0.5 ? "Move the layer below. The island has something buried in it." : "Charred rice survived at early island settlements.")
                        .font(.custom("LibertinusSerif-Regular", size: 18, relativeTo: .body))
                }
            }
            VStack(alignment: .leading, spacing: 5) {
                HStack {
                    Text("SURFACE")
                    Spacer()
                    Text("BENEATH")
                }
                .font(.custom("LibertinusSerif-Regular", size: 12, relativeTo: .caption))
                .tracking(1)
                Slider(value: $time, in: 0...1)
                    .tint(Self.blue)
                    .accessibilityLabel("Layer of evidence")
                    .accessibilityValue(time < 0.5 ? "Present day" : "Early crop traces, about 700 to 1000 CE")
                    .accessibilityIdentifier("evidenceTime")
                Text(time < 0.5 ? "Present day · the landscape holds older traces" : "c. 700–1000 CE · early crop assemblages")
                    .font(.custom("LibertinusSerif-Regular", size: 16, relativeTo: .body))
                    .accessibilityIdentifier("evidenceEra")
            }
            prose("Excavations in Madagascar and the Comoros recovered rice among their early crops. At several tested island sites, Asian crops dominated the preserved assemblages. On the sampled East African coast, the pattern was different.")
                .accessibilityIdentifier("encounterClaim")
            margin("A kitchen leaves a different kind of history from a king.")
            prose("Crowther and colleagues read these crops alongside language and genetic evidence as traces of Southeast Asian settlers in the western Indian Ocean. The surprise is not just that food traveled. People made homes across an ocean, and traces of everyday meals help us recover their presence.")
            note("The layer reveals archaeological evidence, not a simulation of migration. These dates are not a first-arrival date. The grain is an original stylised illustration, not an excavated specimen. These finds do not establish an exact voyage.")
            Button { choose("words") } label: { Text("There is another trace, still spoken  →") }
                .font(.custom("LibertinusSerif-Regular", size: 20, relativeTo: .title3))
                .foregroundStyle(Self.blue).padding(.vertical, 10)
        }
    }

    private var words: some View {
        VStack(alignment: .leading, spacing: 20) {
            Text(borrowed ? "A BORROWED MARITIME WORD" : "TWO RELATIVES, FAR APART")
                .font(.custom("LibertinusSerif-Regular", size: 13, relativeTo: .caption))
                .tracking(1.4).foregroundStyle(Self.blue)
            HStack(alignment: .firstTextBaseline, spacing: 12) {
                word(borrowed ? "sambo" : "zato", language: "Malagasy")
                Text("·").font(.title)
                word(borrowed ? "sambaw" : "jatuh", language: borrowed ? "Old Malay" : "Ma’anyan")
            }
            Text(borrowed ? "Both mean “ship”." : "Both mean “hundred”.")
                .font(.custom("LibertinusSerif-Italic", size: 23, relativeTo: .title2))
            if borrowed {
                note("This pair opens a separate history of contact. The map’s glass keeps the inherited zato / jatuh comparison; it does not locate an Old Malay departure port.")
            }
            prose("Malagasy belongs to the South East Barito branch of the Austronesian language family. Its other members are spoken in southern Borneo, across the Indian Ocean. Madagascar is just off Africa; this part of its linguistic family is not.")
                .accessibilityIdentifier("encounterClaim")
            Button {
                withAnimation(reduceMotion ? nil : .easeInOut(duration: 0.25)) { borrowed.toggle() }
            } label: { Text(borrowed ? "Return to the inherited word  ↶" : "And a word for crossing water?  →") }
                .font(.custom("LibertinusSerif-Regular", size: 20, relativeTo: .title3))
                .foregroundStyle(Self.blue).padding(.vertical, 10)
                .accessibilityIdentifier("wordTangent")
            margin(borrowed ? "A family tree is not the whole story of a language." : "The ocean separates the places. The words preserve a relationship.")
            prose(borrowed ? "Adelaar connects sambo with Old Malay sambaw in the history of maritime borrowing. That is a different relationship from inherited vocabulary: Malagasy did not descend from Malay. Contact left its own traces, including through Bantu languages in Africa." : "The paired forms are a linguist’s comparison, not a recorded conversation. They do not tell us which shore a particular speaker visited, or how many journeys were made. Nor did Malagasy remain unchanged: African and Asian contacts helped shape it.")
            note("The map marks present-day places as orientation, not ancient ethnic territories. Words are shown in scholarly or standard spelling; no reconstructed pronunciation is performed.")
        }
    }

    private var lives: some View {
        VStack(alignment: .leading, spacing: 18) {
            margin("What if we look at the islands between?")
            prose("The Comoros complicate a tidy two-island story. Sima, on Anjouan, yielded rice, mung bean and cotton. Crowther and colleagues interpret the pattern as evidence of a Southeast Asian presence beyond Madagascar. Comorian languages today are Bantu languages.")
                .accessibilityIdentifier("encounterClaim")
            prose("A seed cannot tell us a person’s name. A related word cannot tell us why they left. The early migrants’ status, motives and stages of travel remain uncertain. There is no passenger list hiding behind this map.")
            margin("Madagascar is not a misplaced piece of somewhere else.")
            prose("African and Asian connections belong to Malagasy histories together. Bantu-language contact helped shape Malagasy. A large genetic study found both African and Asian ancestry across its sampled communities, with substantial variation. Neither result defines any person’s identity.")
            note("The map offers places to inspect, not a single migration corridor or a claim that geography determined people’s choices.")
            Button { place = "comoros" } label: { Text("Look at the Comoros  ↗") }
                .font(.custom("LibertinusSerif-Regular", size: 20, relativeTo: .title3))
                .foregroundStyle(Self.blue).padding(.vertical, 10)
        }
    }

    private func word(_ value: String, language: String) -> some View {
        VStack(alignment: .leading, spacing: 6) {
            Text(value).font(.custom("LibertinusSerif-Regular", size: 33, relativeTo: .largeTitle))
                .minimumScaleFactor(0.7).lineLimit(1)
            Text(language).font(.custom("LibertinusSerif-Regular", size: 16, relativeTo: .body))
        }.frame(maxWidth: .infinity, alignment: .leading)
    }

    private func placeNote(_ key: String) -> some View {
        VStack(alignment: .leading, spacing: 8) {
            Text(key == "borneo" ? "Southern Borneo" : key == "comoros" ? "The Comoros" : key == "africa" ? "The East African coast" : "Madagascar")
                .font(.custom("LibertinusSerif-Regular", size: 25, relativeTo: .title2))
            prose(key == "borneo" ? "Other South East Barito languages are spoken near the Barito River. The mark is a regional reference, not a known departure port." : key == "comoros" ? "An archipelago between Africa and Madagascar. Sima’s crop remains are one reason the story reaches beyond Madagascar. The mark is an island-group locator, not a precise excavation coordinate." : key == "africa" ? "The crop study compared island sites with coastal East African sites. Its sampled mainland sites were dominated by African crops; that is a local archaeological comparison, not a timeless cultural boundary." : "At Mahilaka in northwestern Madagascar, rice and cotton were recovered. The island mark locates Madagascar, not the excavation itself.")
            Button("Put this note aside") { place = nil }
                .font(.custom("LibertinusSerif-Regular", size: 17, relativeTo: .body)).foregroundStyle(Self.blue)
                .frame(minHeight: 44)
        }
        .padding(16)
        .background(Self.blue.opacity(0.07))
        .overlay(alignment: .leading) { Rectangle().fill(Self.blue).frame(width: 2) }
    }

    private func prose(_ text: String) -> some View {
        Text(text).font(.custom("LibertinusSerif-Regular", size: 20, relativeTo: .body))
            .lineSpacing(4).fixedSize(horizontal: false, vertical: true)
    }
    private func margin(_ text: String) -> some View {
        Text(text).font(.custom("LibertinusSerif-Italic", size: 26, relativeTo: .title2))
            .foregroundStyle(Self.blue).fixedSize(horizontal: false, vertical: true)
            .padding(.vertical, 8)
    }
    private func note(_ text: String) -> some View {
        Text(text).font(.custom("LibertinusSerif-Regular", size: 16, relativeTo: .footnote))
            .lineSpacing(3).fixedSize(horizontal: false, vertical: true)
    }
    private func choose(_ key: String) {
        withAnimation(reduceMotion ? nil : .easeInOut(duration: 0.35)) { lens = key; place = nil }
    }
}

private struct OceanEvidenceView: View {
    @Environment(\.dismiss) private var dismiss
    var body: some View {
        NavigationStack {
            ScrollView {
                VStack(alignment: .leading, spacing: 24) {
                    Text("What survives, and what it cannot tell us")
                        .font(.custom("LibertinusSerif-Regular", size: 30, relativeTo: .title))
                    Text("These notes are bundled for offline reading. The cited works are not bundled. URLs are selectable references; this encounter makes no network requests. Source check: 26 October 2026 (ET), by an AI research agent, not a human specialist.")
                    ForEach(OceanEvidence.sources) { source in
                        VStack(alignment: .leading, spacing: 10) {
                            Text(source.title).font(.custom("LibertinusSerif-Semibold", size: 23, relativeTo: .title2))
                            Text(source.detail).foregroundStyle(OceanExperience.blue)
                            Text(source.support)
                            Text("Limits · " + source.limitation).italic()
                            Text(source.url).font(.system(.footnote)).textSelection(.enabled)
                        }
                        Divider()
                    }
                }
                .font(.custom("LibertinusSerif-Regular", size: 18, relativeTo: .body))
                .lineSpacing(3)
                .padding(22).frame(maxWidth: 680).frame(maxWidth: .infinity)
            }
            .accessibilityIdentifier("evidenceSheet")
            .background(OceanExperience.paper)
            .foregroundStyle(OceanExperience.ink)
            .navigationTitle("Evidence & open questions")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .confirmationAction) {
                    Button("Done") { dismiss() }.accessibilityIdentifier("closeEvidence")
                }
            }
        }
        .tint(OceanExperience.blue).preferredColorScheme(.light)
    }
}
