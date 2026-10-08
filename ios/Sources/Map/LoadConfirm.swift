import Foundation

enum LoadStep { case Confirm, Load }

func loadStep(_ dirty: Bool, _ armed: Bool) -> LoadStep { dirty && !armed ? .Confirm : .Load }

func replaceWarning(_ items: Int) -> String { "Your unsaved plan here (\(itemCountText(items))) will be replaced." }
