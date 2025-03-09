class App {
    constructor() {
        this.init();
    }

    async init() {
        await this.loadCarriers();
        this.loadParcels();
        this.displayParcels();
        this.initEvents();
        console.log("App initialized");
        console.log(this.carriers);
    }

    async loadCarriers() {
        // fetch from /carriers, is a list of string
        let response = await fetch("/carriers");
        this.carriers = await response.json();
	}

    loadParcels() {
        return Object.values(localStorage).map(JSON.parse);
    }

    loadParcel(parcel) {
        return JSON.parse(localStorage.getItem(parcel));
    }

    hasParcel(parcel) {
        return localStorage.getItem(parcel) !== null;
    }

    addParcel(parcel, data) {
        localStorage.setItem(parcel, JSON.stringify(data));
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
        let parcelItems = parcels.map(this.buildParcelItem);
        parcelList.innerHTML = "";
        parcelItems.forEach((item) => parcelList.appendChild(item));
    }

    async updateParcels() {
        let parcels = this.loadParcels();
        let request = [];
        for (let parcel of parcels) {
            let carriers = parcel.carriers;
            let id = parcel.id;
            request.push([id, carriers]);
        }
        let response = await this.trackParcels(request, navigator.language);
            let result = await response.json();
            if (result.Ok !== null) {
                console.log(result.Ok);
                for (let parcel of result.Ok) {
                    localStorage.setItem(parcel.id, JSON.stringify(parcel));
                }
                this.displayParcels();
            }

    }

    buildParcelItem(parcel) {
        let item = document.createElement("li");
        item.classList.add("parcel-item");
        // display name, carriers, tracking number, last event datetime, status
        let name = parcel.name || "";
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
        let humanLastEvent = parsedLastEvent.toLocaleString(locale, options);
        let humanFirstEvent = parsedFirstEvent.toLocaleString(locale, options);
        item.innerHTML = `
            <div class="parcel-card-line">
                <span class="parcel-id">${id}</span>
                <span class="parcel-status">${parcel.status}</span>
            </div>
            <div class="parcel-card-line"> 
                <span class="parcel-first-event">${humanFirstEvent}</span>
                <span class="parcel-last-event">${humanLastEvent}</span>
            </div>
            <div class="parcel-card-line">
                <span class="parcel-start-region">${parcel.start_region || ""}</span>
                <span class="parcel-end-region">${parcel.end_region}</span>
            </div>
            <div class="parcel-card-line">
                <span class="parcel-product">${parcel.product || ""}</span>
                <span class="parcel-carriers">${parcel.carriers.join(", ")}</span>
            </div>
        `;
        return item;
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
        refreshButton.addEventListener("click", async (_) => await this.updateParcels());
    }

}

window.addEventListener("load", (_) => new App());
