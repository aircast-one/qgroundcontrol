import Foundation

struct CameraChoice: Equatable {
    let brand: String
    let model: String
    let brands: [String]
    let models: [String]

    static let empty = CameraChoice(brand: "", model: "", brands: [], models: [])

    init(brand: String, model: String, brands: [String], models: [String]) {
        self.brand = brand
        self.model = model
        self.brands = brands
        self.models = models
    }

    init(json: [String: Any]) {
        brand = (json["cameraBrand"] as? String) ?? ""
        model = (json["cameraModel"] as? String) ?? ""
        brands = (json["cameraBrandList"] as? [String]) ?? []
        models = (json["cameraModelList"] as? [String]) ?? []
    }

    var canChooseBrand: Bool { brands.count > 1 }

    // Manual and Custom are brands with nothing to pick under them: one means no camera
    // specs at all, the other means the operator types them in.
    var canChooseModel: Bool { models.count > 1 || (models.count == 1 && models.first != model) }

    var describes: String {
        guard !brand.isEmpty else { return "No camera" }
        return model.isEmpty ? brand : "\(brand) \(model)"
    }
}
