import Foundation
import JianyiEngine

final class Jianyi: ExactModule {
    override func later(_ request: [String: Any], reply: ExactReply) {
        guard request["op"] as? String == "cast", let seed = request["seed"] as? String else {
            reply.fail("Unsupported jianyi request")
            return
        }
        do {
            let input = try JSONSerialization.data(withJSONObject: ["timestamp": seed])
            let result = String(decoding: input, as: UTF8.self).withCString { jianyi_iching_cast($0) }
            defer { jianyi_result_free(result) }
            guard result.success, let output = result.data else {
                reply.fail(result.error.map { String(cString: $0) } ?? "Casting engine failed")
                return
            }
            let value = try JSONSerialization.jsonObject(with: Data(String(cString: output).utf8))
            guard let object = value as? [String: Any] else {
                reply.fail("Invalid casting engine response")
                return
            }
            reply.send(object)
        } catch { reply.fail(error.localizedDescription) }
    }
}
let exactModule: ExactModule.Type = Jianyi.self
