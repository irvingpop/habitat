// Copyright (c) 2016-2017 Chef Software Inc. and/or applicable contributors
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

import {Component, Input} from "@angular/core";
import {TabComponent} from "./TabComponent";

@Component({
    selector: "hab-tabs",
    template: `
    <ul class="hab-tabs">
        <li *ngFor="let tab of tabs; let i=index;"
            [ngClass]="{ active: tab.active }"
            [attr.disabled]="tab.formStep && tab.formStep.disabled"
            (click)="selectTab(tab)">{{tab.tabTitle}}</li>
    </ul>
    <ng-content></ng-content>`
})

export class TabsComponent {
    @Input() formSteps: Array<Object>;

    private tabs;

    constructor() {
        this.tabs = [];
    }

    addTab(tab: TabComponent) {
        if (this.tabs.length === 0) { tab.active = true; }
        this.tabs.push(tab);
    }

    selectTab(tab: TabComponent) {
        if (tab.formStep && tab.formStep.disabled) {
            return false;
        }
        this.tabs.forEach(tab => tab.active = false);
        tab.active = true;

        if (typeof tab.onSelect === "function") {
            tab.onSelect();
        }
    }
}
