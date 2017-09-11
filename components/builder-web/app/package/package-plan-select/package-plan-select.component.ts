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

import { Component, Input, OnInit, AfterViewInit } from "@angular/core";
import { FormControl, FormGroup, FormBuilder, Validators } from "@angular/forms";
import { RouterLink } from "@angular/router";
import { MdRadioModule } from "@angular/material";
import { GitHubApiClient } from "../../GitHubApiClient";
import { GitHubFileResponse } from "../../github/api/shared/github-file-response.model";
import { GitHubFile } from "../../github/file/shared/github-file.model";
import { AppStore } from "../../AppStore";
import { addProject, fetchProject, updateProject } from "../../actions/index";
import config from "../../config";

@Component({
  selector: "hab-plan-select",
  template: require("./package-plan-select.component.html")
})

export class PackagePlanSelectComponent implements OnInit {
  @Input() submitCallback: Function;
  @Input() cancelCallback: Function;
  @Input() ownerAndRepo: string;
  @Input() project: string;

  gitHubClient: GitHubApiClient = new GitHubApiClient(this.store.getState().gitHub.authToken);
  form: FormGroup;
  formIndex: number = 0;
  onRepoSelect: Function;
  plans: Array < GitHubFile > ;
  errorText: string;

  disablePlanSelectTab: boolean = true;
  repo: string = "";
  owner: string = "";
  plan: string = "";

  constructor(private formBuilder: FormBuilder, private store: AppStore) {
    this.onRepoSelect = (ownerAndRepo: string) => {
      [this.owner, this.repo] = ownerAndRepo.split("/");

      this.gitHubClient.findFileInRepo(this.owner, this.repo, "plan.")
        .then(this.handleGitHubFileResponse);

      return false;
    };
  }

  get token() {
    return this.store.getState().gitHub.authToken;
  }

  onTabChange(tab) {
    this.formIndex = tab.index;
  }

  handleGitHubFileResponse(result: GitHubFileResponse) {
    if (result.total_count === 0) {
      this.owner = "";
      this.repo = "";
      this.errorText = "That repo doesn't appear to have a plan file. Please select another repo.";
      return false;
    }

    this.errorText = "";
    this.formIndex = 1;

    this.plans = result.items.map((item) => {
      if (item.name.endsWith(".sh")) {
        item.type = "linux";
      } else if (item.name.endsWith(".ps1")) {
        item.type = "windows";
      }

      return item;
    });
  }

  handleSubmit() {
    console.log("submit");
    this.submitCallback("submitted");
    // Change the format to match what the server wants
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

  public ngOnInit() {
    this.form = this.formBuilder.group({
      repo: [this.repo || "", Validators.required]
    });
  }
}