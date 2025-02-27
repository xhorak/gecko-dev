/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

"use strict";

var tempElements = [];

function insertTempItemsIntoMenu(parentMenu) {
  // Last element is null to insert at the end:
  let beforeEls = [parentMenu.firstChild, parentMenu.lastChild, null];
  for (let i = 0; i < beforeEls.length; i++) {
    let sep = document.createElement("menuseparator");
    tempElements.push(sep);
    parentMenu.insertBefore(sep, beforeEls[i]);
    let menu = document.createElement("menu");
    tempElements.push(menu);
    parentMenu.insertBefore(menu, beforeEls[i]);
    // And another separator for good measure:
    sep = document.createElement("menuseparator");
    tempElements.push(sep);
    parentMenu.insertBefore(sep, beforeEls[i]);
  }
}

function checkSeparatorInsertion(menuId, buttonId, subviewId) {
  return async function() {
    info("Checking for duplicate separators in " + buttonId + " widget");
    let menu = document.getElementById(menuId);
    insertTempItemsIntoMenu(menu);

    CustomizableUI.addWidgetToArea(buttonId, CustomizableUI.AREA_FIXED_OVERFLOW_PANEL);
    await document.getElementById("nav-bar").overflowable.show();

    let button = document.getElementById(buttonId);
    button.click();

    await BrowserTestUtils.waitForEvent(PanelUI.overflowPanel, "ViewShown");
    let subview = document.getElementById(subviewId);
    ok(subview.firstChild, "Subview should have a kid");
    is(subview.firstChild.localName, "toolbarbutton", "There should be no separators to start with");

    for (let kid of subview.children) {
      if (kid.localName == "menuseparator") {
        ok(kid.previousSibling && kid.previousSibling.localName != "menuseparator",
           "Separators should never have another separator next to them, and should never be the first node.");
      }
    }

    let panelHiddenPromise = promiseOverflowHidden(window);
    PanelUI.overflowPanel.hidePopup();
    await panelHiddenPromise;

    CustomizableUI.reset();
  };
}

add_task(checkSeparatorInsertion("menuWebDeveloperPopup", "developer-button", "PanelUI-developerItems"));

registerCleanupFunction(function() {
  for (let el of tempElements) {
    el.remove();
  }
  tempElements = null;
});
