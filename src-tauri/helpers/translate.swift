// newdawn-translate: Apple's on-device Translation framework for New Dawn (D-046). Bundled beside
// the app (Tauri externalBin) and run by `kernel/ai/translate.rs`; nothing leaves the Mac.
//
//   newdawn-translate status vi fr …    → {"vi": {"to_en": "installed", "from_en": "supported"}, …}
//   newdawn-translate translate vi en   ← stdin: ["text", …]   → stdout: ["translation", …]
//
// Exit 0 with JSON on stdout; otherwise exit 1 with a short reason on stderr (never the text).
import Foundation
import Translation

func fail(_ reason: String) -> Never {
    FileHandle.standardError.write(Data((reason + "\n").utf8))
    exit(1)
}

func emit(_ value: Any) {
    guard let data = try? JSONSerialization.data(withJSONObject: value) else { fail("couldn't write the answer") }
    FileHandle.standardOutput.write(data)
}

@available(macOS 15.0, *)
func statusName(_ status: LanguageAvailability.Status) -> String {
    switch status {
    case .installed: return "installed"
    case .supported: return "supported"
    case .unsupported: return "unsupported"
    @unknown default: return "unsupported"
    }
}

#if compiler(>=6.2) // Xcode 26: the macOS 26 SDK, sessions without SwiftUI
@available(macOS 26.0, *)
func session(from source: Locale.Language, to target: Locale.Language) -> TranslationSession {
    #if compiler(>=6.3) // Xcode 26.4: translation strategies
    // The Apple Intelligence model where there is one (macOS 26.4+), else the system's choice.
    if #available(macOS 26.4, *) {
        return TranslationSession(installedSource: source, target: target, preferredStrategy: .highFidelity)
    }
    #endif
    return TranslationSession(installedSource: source, target: target)
}
#endif

@main struct Main {
    static func main() async {
        let args = Array(CommandLine.arguments.dropFirst())
        guard #available(macOS 26.0, *) else { fail("Translation needs macOS 26 or later") }
        switch args.first {
        case "status":
            let english = Locale.Language(identifier: "en")
            let availability = LanguageAvailability()
            var out: [String: [String: String]] = [:]
            for code in args.dropFirst() {
                let other = Locale.Language(identifier: code)
                out[code] = [
                    "to_en": statusName(await availability.status(from: other, to: english)),
                    "from_en": statusName(await availability.status(from: english, to: other)),
                ]
            }
            emit(out)
        case "translate" where args.count == 3:
            #if compiler(>=6.2)
            let input = FileHandle.standardInput.readDataToEndOfFile()
            guard let texts = (try? JSONSerialization.jsonObject(with: input)) as? [String] else {
                fail("the input was not a JSON list of strings")
            }
            let translator = session(from: Locale.Language(identifier: args[1]), to: Locale.Language(identifier: args[2]))
            let requests = texts.enumerated()
                .filter { !$0.element.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty }
                .map { TranslationSession.Request(sourceText: $0.element, clientIdentifier: String($0.offset)) }
            var out = texts
            do {
                for response in try await translator.translations(from: requests) {
                    if let id = response.clientIdentifier, let i = Int(id) { out[i] = response.targetText }
                }
            } catch {
                // The error's kind only: its details could carry the text (D-026).
                let kind = (error as NSError)
                fail("translation failed: \(kind.domain) \(kind.code)")
            }
            emit(out)
            #else
            fail("This build can't translate (made without the macOS 26 SDK)")
            #endif
        default:
            fail("usage: newdawn-translate status <lang>… | translate <from> <to> < [texts]")
        }
    }
}
