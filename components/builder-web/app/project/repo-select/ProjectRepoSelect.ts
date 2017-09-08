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

import {Component, Input, OnInit, AfterViewInit} from "@angular/core";
import {FormControl, FormGroup, FormBuilder, Validators} from "@angular/forms";
import {GitHubApiClient} from "../../GitHubApiClient";
import {GitHubFileResponse} from "../../github/api/shared/github-file-response.model";
import {GitHubFile} from "../../github/file/shared/github-file.model";
import {AppStore} from "../../AppStore";
import {addProject, fetchProject, updateProject} from "../../actions/index";
import {RouterLink} from "@angular/router";
import config from "../../config";
import {setRedirectRoute} from "../../actions/index";


@Component({
    selector: "hab-project-repo-select",
    template: `
    <div class="has-sidebar">
      <div class="page-body--main">
          <div class="page-body has-sidebar">
            <hab-tabs>
              <hab-tab tabTitle="1. Select a GitHub repo">
                <hab-scm-repos></hab-scm-repos>
              <hab-tab>
              <hab-tab tabTitle="2. Set path to Habitat plan file">
                plan
              </hab-tab>
            </hab-tabs>
            <hab-tabs>
              <hab-tab *ngIf="build" tabTitle="Build Output">
                  <div class="page-body has-sidebar">
                      <hab-build [build]="build" stream="false"></hab-build>
                  </div>
              </hab-tab>
              <hab-tab tabTitle="Manifest">
                  <div class="page-body has-sidebar">
                      <hab-package-info [package]="package"></hab-package-info>
                  </div>
              </hab-tab>
          </hab-tabs>
          </div>
      </div>
      <div class="page-body--sidebar">
          <h3>Plans and exports</h3>
          <p>While you can connect many plans to the Build Service, not that the origin and package name in each selected plan file must be <strong>myorigin</strong> and <strong>testapp</strong> respectively.</p>
          <p>For example, you might have both a plan.sh and a plan.ps1 if this package supports both Linux and Windows platforms, but both will contain the same origin and package name.</p>
          <p>Finally, you have the option to automatically export your Linux .hart files as Docker containers and publish them to Docker Hub.</p>
      </div>
  </div>
    `
//     <ul class="hab-plans-list">
//     <li *ngFor="let plan of plans">
//         <label>
//             <input [ngClass]="['hab-form-radio', 'hab-radio-item-' + plan.type]"
//                 type="radio" formControlName="selectedPlan" ng-value="plan.path">
//             {{plan.path}}
//         </label>
//     </li>
// </ul>
    // template: `
    // <form [formGroup]="form" (ngSubmit)="submitProject(form.value)" #formValues="ngForm">
    //   <div class="scm-repo-fields">
    //       <label>GitHub Repository</label>
          // <div *ngIf="repo">
          //     <a href="${config["github_web_url"]}/{{ownerAndRepo}}" target="_blank">
          //         {{ownerAndRepo}}
          //     </a>
          //     <a [routerLink]="['/scm-repos']" href="#">(change)</a>
          // </div>
          // <div *ngIf="!repo">
          //     <a [routerLink]="['/scm-repos']" href="#">
          //         (select a GitHub repository)
          //     </a>
          // </div>
    //   </div>
    //   <div class="project-fields">
    //       <div class="plan">
    //           <label for="plan">Select a path file</label>
    //           <small>Enter a path to a plan.sh file, or select one below:</small>
    //           <hab-checking-input availableMessage="exists"
    //                               displayName="File"
    //                               [form]="form"
    //                               id="plan"
    //                               [isAvailable]="doesFileExist"
    //                               [maxLength]="false"
    //                               name="plan_path"
    //                               notAvailableMessage="does not exist in repository"
    //                               [pattern]="false"
    //                               [value]="planPath">
    //           </hab-checking-input>
    //           <ul class="hab-plans-list">
    //             <li *ngFor="let plan of plans">
    //                 <a
    //                 class="hab-item-list">
    //                     <div class="hab-item-list--title">
    //                         <h3>{{plan.path}}</h3>
    //                     </div>
    //                 </a>
    //             </li>
    //           </ul>
    //       </div>
    //         <div class="submit">
    //             <button type="submit" [disabled]="!form.valid">
    //                 Save Project
    //             </button>
    //         </div>
    //     </div>
    // </form>
    // `
})

export class ProjectInfoComponent implements AfterViewInit, OnInit {
    form: FormGroup;
    doesFileExist: Function;
    plans: Array<GitHubFile>;

    selectedPlan: string;

    formSteps = [
        { target: "", name: "1. Select a GitHub repo", current: true, disabled: true },
        { target: "", name: "2. Set path to Habitat plan file", disabled: true }
    ];

    @Input() project: Object;
    @Input() ownerAndRepo: String;

    constructor(private formBuilder: FormBuilder, private store: AppStore) {}

    get repoOwner() {
        return (this.ownerAndRepo || "/").split("/")[0];
    }

    get repo() {
        return (this.ownerAndRepo || "/").split("/")[1];
    }

    get token() {
        return this.store.getState().gitHub.authToken;
    }

    submitProject(values) {
        // Change the format to match what the server wants
        console.log(values);
        console.log(this.selectedPlan);
        // values.github = {
        //     organization: this.repoOwner,
        //     repo: this.repo
        // };

        // let hint = this.store.getState().projects.hint;
        // values.origin = hint["originName"];

        // delete values.repo;

        // let rr;
        // let currentPackage = this.store.getState().packages.current;

        // if (this.redirectRoute) {
        //     rr = this.redirectRoute;
        // } else if (currentPackage === undefined || currentPackage.ident.origin === undefined) {
        //     rr = ["origins", values["origin"]];
        // } else {
        //     rr = [
        //         "pkgs",
        //         currentPackage.ident.origin,
        //         currentPackage.ident.name,
        //         currentPackage.ident.version,
        //         currentPackage.ident.release
        //     ];
        // }

        // if (this.project) {
        //     this.store.dispatch(updateProject(this.project["id"], values, this.token, rr));
        // } else {
        //     this.store.dispatch(addProject(values, this.token, rr));
        // }

        // return false;
    }

    ngAfterViewInit() {
        // Wait a second to set the fields as dirty to do validation on page
        // load. Doing this later in the lifecycle causes a changed after it was
        // // checked error.
        // setTimeout(() => {
        //     this.form.controls["plan_path"].markAsDirty();
        //  } , 1000);
    }

    public ngOnInit() {
      // this.store.dispatch(setRedirectRoute(["/origins", this.origin]));
    }
}
