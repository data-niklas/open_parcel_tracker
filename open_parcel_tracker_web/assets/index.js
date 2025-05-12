const zip = (arr1, arr2) => arr1.map((element, index) => [element, arr2[index]]);

const EMPTY = "N/A";

class App {
    constructor() {
        this.init();
    }

    async init() {
        this.updateInterval = 1000 * 60 * 15; // 15 minutes
        await this.loadCarriers();
        this.loadParcels();
        this.displayParcels();
        this.initEvents();
        this.initEscapeKeyBack();
        console.log("App initialized");
        console.log(this.carriers);
    }

    async loadCarriers() {
        // fetch from /carriers, is a list of string
        let response = await fetch("/carriers");
        this.carriers = await response.json();
	}

    loadParcels() {
        return Object.values(localStorage).map(JSON.parse).map((parcel) => {
            parcel.addTime = new Date(parcel.addTime);
            return parcel;
        });
    }

    loadParcel(parcel) {
        let parsed_parcel = JSON.parse(localStorage.getItem(parcel));
        parsed_parcel.addTime = new Date(parsed_parcel.addTime);
        return parsed_parcel;
    }

    hasParcel(parcel) {
        return localStorage.getItem(parcel) !== null;
    }

    addParcel(parcel_id, parcel) {
        parcel.addTime = new Date().toISOString();
        localStorage.setItem(parcel_id, JSON.stringify(parcel));
    }


    async trackParcels(parcels, language) {
        return await fetch("/track", {
            method: "POST",
            headers: {
                "Content-Type": "application/json",
            },
            body: JSON.stringify({
                parcels: parcels,
                language: language,
            }),
        });
    }

    displayParcels() {
        let parcelList = document.getElementById("parcel-list");
        let parcels = this.loadParcels();
        // sort by events[events.length - 1].datetime ascending
        parcels.sort((a, b) => {
            let aEvent = a.events[a.events.length - 1];
            let bEvent = b.events[b.events.length - 1];
            return aEvent.datetime.localeCompare(bEvent.datetime);
        });
        let parcelItems = parcels.filter((parcel)=>{
            return parcel.archived === undefined || parcel.archived === false;
        }).map((parcel)=>this.buildParcelItem(parcel, true));
        parcelList.innerHTML = "";
        parcelItems.forEach((item) => parcelList.appendChild(item));

        let parcelListArchive = document.getElementById("parcel-list-archive");
        let archivedParcels = parcels.filter((parcel)=>{
            return parcel.archived === true;
        }).map((parcel)=>this.buildParcelItem(parcel, true));
        parcelListArchive.innerHTML = "";
        archivedParcels.forEach((item) => parcelListArchive.appendChild(item));
    }

    setMaybeParcel(parcel, parcel_id) {
        if (parcel === null) {
            localStorage.removeItem(parcel_id);
            return;
        }
        this.addParcel(parcel_id, parcel);
    }

    async update(){
        if (this.view) {
            this.updateParcels();
        }
        else {
            this.updateParcel();
        }
    }

    async updateParcel() {
        let parcel = this.loadParcel(this.currentParcelId);
        let carriers = parcel.carriers;
        let id = parcel.id;
        let response = await this.trackParcels([[id, carriers]], navigator.language);
        let result = await response.json();
        if (result.Ok !== null) {
            console.log(result.Ok[0]);
            this.setMaybeParcel(result.Ok[0], id);
            this.displayParcelDetails(id);
        }
    }


    async updateParcels() {
        let parcels = this.loadParcels();
        let request = [];
        let now = new Date();
        for (let parcel of parcels) {
            if (now - parcel.addTime < this.updateInterval) {
                console.log("Skipping parcel due to time", parcel.id);
                continue;
            }
            if (parcel.archived) {
                console.log("Skipping archived parcel", parcel.id);
                continue;
            }
            let carriers = parcel.carriers;
            let id = parcel.id;
            request.push([id, carriers]);
        }
        if (request.length === 0) {
            return;
        }
        let response = await this.trackParcels(request, navigator.language);
            let result = await response.json();
            if (result.Ok !== null) {
                console.log(result.Ok);
                let parcel_with_id_and_carrier = zip(result.Ok, request);
                for (let parcel of parcel_with_id_and_carrier) {
                    this.setMaybeParcel(parcel[0], parcel[1][0]);
                }
                this.displayParcels();
            }

    }

    buildParcelItem(parcel, withActions) {
        let archived = parcel.archived || false;
        let item = document.createElement("li");
        item.classList.add("parcel-item");
        // display name, carriers, tracking number, last event datetime, status
        let name = parcel.name || EMPTY;
        let id = parcel.id;
        if (parcel.name) {
            id = `${parcel.name} (${id})`;
        }
        let lastEvent = parcel.events[0];
        let firstEvent = parcel.events[parcel.events.length - 1];
        let parsedLastEvent = new Date(lastEvent.datetime);
        let parsedFirstEvent = new Date(firstEvent.datetime);
        // human readable datetime
        const options = {
  year: 'numeric',
  month: '2-digit',
  day: '2-digit',
  hour: '2-digit',
  minute: '2-digit',
  hour12: false,
};
        const locale = navigator.language;
        let humanLastEvent = parsedLastEvent.toLocaleString("en-CA", options);
        let humanFirstEvent = parsedFirstEvent.toLocaleString("en-CA", options);
        let archiveText = archived ? "Unarchive" : "Archive";
        item.innerHTML = `
        <div class="parcel-info">
            <div class="parcel-card-line">
                <span class="parcel-status">${parcel.status}</span>
                <span class="parcel-id">${id}</span>
            </div>
            <table>
                <tr>
                    <td class="table-key">Tracked since:</td><td class="table-value">${humanFirstEvent}</td>
                    <td class="table-key">Last event:</td><td class="table-value">${humanLastEvent}</td>
                </tr>
                <tr>
                    <td class="table-key">Start region:</td><td class="table-value">${parcel.start_region || EMPTY}</td>
                    <td class="table-key">End region:</td><td class="table-value">${parcel.end_region}</td>
                </tr>
                <tr>
                    <td class="table-key">Product:</td><td class="table-value">${parcel.product || EMPTY}</td>
                    <td class="table-key">Carriers:</td><td class="table-value">${parcel.carriers.join(", ")}</td>
            </table>
        </div>
        `;
        if (withActions) {
            let actions = document.createElement("div");
            actions.classList.add("parcel-actions");
            actions.innerHTML = `
            <button class="parcel-go">Go</button>
            <button class="parcel-archive">${archiveText}</button>
            <button class="parcel-delete">Delete</button>
            `;
            item.appendChild(actions);
        let goButton = item.querySelector(".parcel-go");
        let archiveButton = item.querySelector(".parcel-archive");
        let deleteButton = item.querySelector(".parcel-delete");
        goButton.addEventListener("click", (_) => {
            this.displayParcelDetails(parcel.id);
            this.switchView(false);
        });
        archiveButton.addEventListener("click", (_) => {
            parcel.archived = !archived;
            localStorage.setItem(parcel.id, JSON.stringify(parcel));
            this.displayParcels();
        });
        deleteButton.addEventListener("click", (_) => {
            this.deleteAction(parcel.id);
        });
        }
        return item;
    }

    createDivider(text) {
        let divider = document.createElement("div");
        divider.classList.add("divider");
        divider.innerHTML = `
            <span>${text}</span>
        `;
        return divider;
    }

    deleteAction(parcelId) {
        let deleteDialog = document.getElementById("confirm-delete-dialog");
        let deleteButton = document.getElementById("confirm-delete");
        let cancelButton = document.getElementById("cancel-delete");
        deleteButton.addEventListener("click", (_) => {
            localStorage.removeItem(parcelId);
            this.displayParcels();
            deleteDialog.close();
        });
        cancelButton.addEventListener("click", (_) => {
            deleteDialog.close();
        });
        deleteDialog.showModal();
    }

    displayParcelDetails(parcelId) {
        let parcel = this.loadParcel(parcelId);
        let parcelDetails = document.getElementById("parcel-events");
        parcelDetails.innerHTML = "";
        let parcelCard = this.buildParcelItem(parcel, false);
        parcelDetails.appendChild(parcelCard);
        parcelDetails.appendChild(this.createDivider("Events"));
        parcel.events.forEach((event)=>{
            parcelDetails.appendChild(this.buildParcelEvent(event));
        });
        this.currentParcelId = parcelId;
    }


    buildParcelEvent(event) {
        // datetime, optional region, description, carrier
        let item = document.createElement("li");
        item.classList.add("parcel-event");
        let parsedEvent = new Date(event.datetime);
        const options = {
  year: 'numeric',
  month: '2-digit',
  day: '2-digit',
  hour: '2-digit',
  minute: '2-digit',
  hour12: false,
};
        const locale = navigator.language;
        let humanEvent = parsedEvent.toLocaleString("en-CA", options);
        item.innerHTML = `
            <div class="parcel-card-line">
                <span class="parcel-event-description">${event.description}</span>
                <span class="parcel-event-region">${event.region || ""}</span>
            </div>
            <div class="parcel-card-line">
                <span class="parcel-event-datetime">${humanEvent}</span>
                <span class="parcel-event-carrier">${event.carrier}</span>
            </div>
        `;
        return item;
    }


    switchView(firstView){
        let parcelList = document.getElementById("parcel-list");
        let parcelListArchive = document.getElementById("parcel-list-archive");
        let archiveTitle = document.getElementById("archive-title");
        let parcelDetails = document.getElementById("parcel-events");
        let addParcel = document.getElementById("addParcel");
        let backButton = document.getElementById("back");
        this.view = firstView;
        if (firstView){
            parcelList.style.display = "block";
            parcelListArchive.style.display = "block";
            archiveTitle.style.display = "flex";
            parcelDetails.style.display = "none";
            addParcel.style.display = "block";
            backButton.style.display = "none";
        }
        else {
            parcelList.style.display = "none";
            parcelListArchive.style.display = "none";
            archiveTitle.style.display = "none";
            parcelDetails.style.display = "block";
            addParcel.style.display = "none";
            backButton.style.display = "block";
        }
    }

    initEscapeKeyBack() {
        document.addEventListener("keydown", (event) => {
            if (event.key === "Escape") {
                if (!this.view) {
                    this.switchView(true);
                }
            }
        });
    }


    initEvents() {
        let parcelInput = document.getElementById("addParcel");
        // type input type=text
        parcelInput.addEventListener("keypress", async (event) => {
            if (event.key != "Enter") {
                return;
            }
            event.preventDefault();
            let parcelCode = parcelInput.value;
            parcelInput.value = "";
            if (this.hasParcel(parcelCode)) {
                console.log("Parcel already added");
                return;
            }
            let response = await this.trackParcels([[parcelCode, this.carriers]], navigator.language);
            let result = await response.json();
            if (result.Ok !== null) {
                console.log(result.Ok[0]);
                this.addParcel(parcelCode, result.Ok[0]);
                this.displayParcels();
            }
        });
        let refreshButton = document.getElementById("refreshButton");
        refreshButton.addEventListener("click", async (_) => await this.update());
        let backButton = document.getElementById("back");
        backButton.addEventListener("click", (_) => this.switchView(true));
    }

}

window.addEventListener("load", (_) => new App());
