import Foundation

struct CameraChoice: Equatable {
    let brand: String
    let model: String
    let brands: [String]
    let shownManual: String
    let shownCustom: String
    let models: [String]
    let isCustom: Bool

    static let empty = CameraChoice(brand: "", model: "", brands: [], models: [],
                                    isCustom: false)

    // CameraCalc stores the CANONICAL name for these two and lists the tr()'d one. Both are
    // marked "This string should NOT be translated" in CameraCalc.cc, so holding them as English
    // literals is correct here and is the one place in this head where that is true.
    static let canonicalManual = "Manual (no camera specs)"
    static let canonicalCustom = "Custom Camera"

    init(brand: String, model: String, brands: [String], models: [String], isCustom: Bool,
         shownManual: String = "", shownCustom: String = "") {
        self.brand = brand
        self.model = model
        self.brands = brands
        self.models = models
        self.isCustom = isCustom
        self.shownManual = shownManual
        self.shownCustom = shownCustom
    }

    init(json: [String: Any]) {
        brand = (json["cameraBrand"] as? String) ?? ""
        model = (json["cameraModel"] as? String) ?? ""
        brands = (json["cameraBrandList"] as? [String]) ?? []
        shownManual = (json["xlatManualCameraName"] as? String) ?? ""
        shownCustom = (json["xlatCustomCameraName"] as? String) ?? ""
        models = (json["cameraModelList"] as? [String]) ?? []
        isCustom = (json["isCustomCamera"] as? NSNumber)?.boolValue ?? false
    }

    // _setBrandModelFromCanonicalName leaves _cameraBrand at the canonical name for a manual or
    // custom camera, while _cameraBrandList carries the translated one, so outside English the
    // picker's selection matches no tag and the control draws BLANK - on every survey using a
    // manual camera, which is what a new survey starts as.
    var selectedBrand: String {
        guard !brands.contains(brand) else { return brand }
        if brand == CameraChoice.canonicalManual, !shownManual.isEmpty { return shownManual }
        if brand == CameraChoice.canonicalCustom, !shownCustom.isEmpty { return shownCustom }
        return brand
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
