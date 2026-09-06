import QtQuick

import QGroundControl.AppSettings

Item {
    id: root

    function matchedNames(filter) {
        return pagesModel.pages()
                         .filter(page => pagesModel.matches(page, filter))
                         .map(page => page.name)
    }

    function visiblePageNames() {
        return pagesModel.pages()
                         .filter(page => page.pageVisible())
                         .map(page => page.name)
    }

    function matchesSyntheticPage(visible, name, filter) {
        return pagesModel.matches({
            name:        name,
            summary:     "",
            keywords:    "",
            pageVisible: function() { return visible }
        }, filter)
    }

    SettingsPagesModel { id: pagesModel }
}
