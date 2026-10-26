// Offline evidence notes. These are paraphrases; the articles themselves are not bundled.
struct OceanSource: Identifiable {
    let id: String
    let title: String
    let detail: String
    let url: String
    let support: String
    let limitation: String
}

enum OceanEvidence {
    static let sources: [OceanSource] = [
        OceanSource(
            id: "language",
            title: "The language across the ocean",
            detail: "Alexander Adelaar · 2017 · Spirits and Ships, pp. 441–469",
            url: "https://www.cambridge.org/core/books/abs/spirits-and-ships/who-were-the-first-malagasy-and-what-did-they-speak/FB1D39086A9D294A71E2CE42E018F01D",
            support: "The publisher's accessible introduction to ‘Who Were the First Malagasy, and What Did They Speak?’ places Malagasy in the South East Barito subgroup, whose other languages are predominantly spoken near the eastern side of the Barito River in southern Borneo. It also identifies Malay, Javanese, Ngaju and South Sulawesi contacts, and Sanskrit loans mediated through Malay and Javanese.",
            limitation: "A language family is a historical relationship, not a map of exclusive ancestry. The introduction was checked; the full subscription chapter was not. Its proposed early migration date is not used here as a settled arrival date."
        ),
        OceanSource(
            id: "words",
            title: "What the words can tell us",
            detail: "Alexander Adelaar · 2016 · NUSA 61, pp. 69–88",
            url: "https://tufs.repo.nii.ac.jp/record/1402/files/04Adelaar.pdf",
            support: "Table 1, printed p. 72, compares Ma’anyan jatuh and Malagasy zàtu, both meaning ‘hundred’. This encounter restores standard Malagasy spelling: zato. Section 8, p. 84, lists Malagasy sàmbu (standard sambo) and Old Malay sambaw, both ‘ship’, in a discussion of language contact. Page 70 explains the author's spelling conventions. Sections 5 and 7 examine Bantu contact and migration debates.",
            limitation: "Shared inherited words and borrowed words are different evidence. Malagasy did not descend from Malay. Similar-looking words alone cannot reconstruct a voyage. The early migrants' identities, social status and motives remain uncertain; no named traveler or recorded dialogue is supplied by this paper."
        ),
        OceanSource(
            id: "seeds",
            title: "The meals that survived as seeds",
            detail: "Alison Crowther and colleagues · 2016 · PNAS 113, pp. 6635–6640",
            url: "https://doi.org/10.1073/pnas.1522714113",
            support: "Table 1 and Results and Discussion (pp. 6636–6638) report rice-dominated early crop assemblages at tested Comorian and Malagasy sites, dating broadly to the eighth–tenth centuries CE. Sima in the Comoros yielded rice, mung bean and cotton; Mahilaka in northwestern Madagascar yielded rice and cotton. The Comoros discussion, p. 6639, interprets the findings alongside linguistic and genetic evidence as support for a Southeast Asian presence beyond Madagascar.",
            limitation: "These are preserved remains from sampled sites, not a census of every meal. Mung bean was not found at Mahilaka. The Asian cotton identification is inferred, since the seeds alone do not distinguish the species. The authors favor a Southeast Asian introduction; rice and mung bean also have South Asian histories. The finds do not fix a route, first arrival date, or the language of any individual."
        ),
        OceanSource(
            id: "people",
            title: "A shared history, many lives",
            detail: "Denis Pierron and colleagues · 2017 · PNAS 114, E6498–E6506",
            url: "https://doi.org/10.1073/pnas.1704906114",
            support: "The abstract and ‘Origin of Malagasy Genomic Diversity’ report a study of 2,704 individuals from 257 villages. All sampled individuals had African and Asian ancestry components, with substantial variation. The Discussion and Figure 6 propose several geographically different histories of mixing, rather than one already-mixed founding population.",
            limitation: "Population genetics cannot define someone's culture or identity, or recover an early voyager's decisions. The migration model is the authors' interpretation and differs from some linguistic reconstructions. This encounter uses no ancestry percentages, individual genetic data, or ethnic territory map."
        )
    ]
}
