let autoRefreshIntervalId = null;
let initialized = false;
let optimizing = false;
let demoDataId = null;
let scheduleId = null;
let loadedRoutePlan = null;
let newVisit = null;
let visitMarker = null;
let routeGeometries = null;  // Cache for encoded polyline geometries
let useRealRoads = true;     // Routing mode toggle state (default: real roads)
const solveButton = $("#solveButton");
const stopSolvingButton = $("#stopSolvingButton");
const vehiclesTable = $("#vehicles");
const analyzeButton = $("#analyzeButton");

/**
 * Decode an encoded polyline string into an array of [lat, lng] coordinates.
 * This is the Google polyline encoding algorithm.
 * @param {string} encoded - The encoded polyline string
 * @returns {Array<Array<number>>} Array of [lat, lng] coordinate pairs
 */
function decodePolyline(encoded) {
  if (!encoded) return [];

  const points = [];
  let index = 0;
  let lat = 0;
  let lng = 0;

  while (index < encoded.length) {
    // Decode latitude
    let shift = 0;
    let result = 0;
    let byte;
    do {
      byte = encoded.charCodeAt(index++) - 63;
      result |= (byte & 0x1f) << shift;
      shift += 5;
    } while (byte >= 0x20);
    const dlat = (result & 1) ? ~(result >> 1) : (result >> 1);
    lat += dlat;

    // Decode longitude
    shift = 0;
    result = 0;
    do {
      byte = encoded.charCodeAt(index++) - 63;
      result |= (byte & 0x1f) << shift;
      shift += 5;
    } while (byte >= 0x20);
    const dlng = (result & 1) ? ~(result >> 1) : (result >> 1);
    lng += dlng;

    // Polyline encoding uses precision of 5 decimal places
    points.push([lat / 1e5, lng / 1e5]);
  }

  return points;
}

/**
 * Fetch route geometries for the current schedule from the backend.
 * @returns {Promise<Object|null>} The geometries object or null if unavailable
 */
async function fetchRouteGeometries() {
  if (!scheduleId) return null;

  try {
    const response = await fetch(`/route-plans/${scheduleId}/geometry`);
    if (response.ok) {
      const data = await response.json();
      // Transform segments array into map: { vehicleId: [polyline] }
      const geometries = {};
      for (const segment of data.segments || []) {
        const vehicleId = String(segment.vehicle_idx);
        if (!geometries[vehicleId]) {
          geometries[vehicleId] = [];
        }
        geometries[vehicleId].push(segment.polyline);
      }
      return Object.keys(geometries).length > 0 ? geometries : null;
    }
  } catch (e) {
    console.warn('Could not fetch route geometries:', e);
  }
  return null;
}

/*************************************** Loading Overlay Functions **************************************/

function showLoadingOverlay(title = "Loading Demo Data", message = "Initializing...") {
  $("#loadingTitle").text(title);
  $("#loadingMessage").text(message);
  $("#loadingProgress").css("width", "0%");
  $("#loadingDetail").text("");
  $("#loadingOverlay").removeClass("hidden");
}

function hideLoadingOverlay() {
  $("#loadingOverlay").addClass("hidden");
}

function updateLoadingProgress(message, percent, detail = "") {
  $("#loadingMessage").text(message);
  $("#loadingProgress").css("width", `${percent}%`);
  $("#loadingDetail").text(detail);
}

/**
 * Load demo data with progress updates via Server-Sent Events.
 * Used when Real Roads mode is enabled.
 */
function loadDemoDataWithProgress(demoId) {
  return new Promise((resolve, reject) => {
    const routingMode = useRealRoads ? "real_roads" : "haversine";
    const url = `/demo-data/${demoId}/stream?routing=${routingMode}`;

    showLoadingOverlay(
      useRealRoads ? "Loading Real Road Data" : "Loading Demo Data",
      "Connecting..."
    );

    const eventSource = new EventSource(url);
    let solution = null;

    eventSource.onmessage = function(event) {
      try {
        const data = JSON.parse(event.data);

        if (data.event === "progress") {
          let statusIcon = "";
          if (data.phase === "network") {
            statusIcon = '<i class="fas fa-download me-2"></i>';
          } else if (data.phase === "routes") {
            statusIcon = '<i class="fas fa-route me-2"></i>';
          } else if (data.phase === "complete") {
            statusIcon = '<i class="fas fa-check-circle me-2 text-success"></i>';
          }
          updateLoadingProgress(data.message, data.percent, data.detail || "");
        } else if (data.event === "complete") {
          solution = data.solution;
          // Store geometries from the response if available
          if (data.geometries) {
            routeGeometries = data.geometries;
          }
          eventSource.close();
          hideLoadingOverlay();
          resolve(solution);
        } else if (data.event === "error") {
          eventSource.close();
          hideLoadingOverlay();
          reject(new Error(data.message));
        }
      } catch (e) {
        console.error("Error parsing SSE event:", e);
      }
    };

    eventSource.onerror = function(error) {
      eventSource.close();
      hideLoadingOverlay();
      reject(new Error("Connection lost while loading data"));
    };
  });
}

/*************************************** Map constants and variable definitions  **************************************/

const homeLocationMarkerByIdMap = new Map();
const visitMarkerByIdMap = new Map();

const map = L.map("map", { doubleClickZoom: false }).setView(
  [51.505, -0.09],
  13,
);
const visitGroup = L.layerGroup().addTo(map);
const homeLocationGroup = L.layerGroup().addTo(map);
const routeGroup = L.layerGroup().addTo(map);

/************************************ Time line constants and variable definitions ************************************/

let byVehicleTimeline;
let byVisitTimeline;
const byVehicleGroupData = new vis.DataSet();
const byVehicleItemData = new vis.DataSet();
const byVisitGroupData = new vis.DataSet();
const byVisitItemData = new vis.DataSet();

const byVehicleTimelineOptions = {
  timeAxis: { scale: "hour" },
  orientation: { axis: "top" },
  xss: { disabled: true }, // Items are XSS safe through JQuery
  stack: false,
  stackSubgroups: false,
  zoomMin: 1000 * 60 * 60, // A single hour in milliseconds
  zoomMax: 1000 * 60 * 60 * 24, // A single day in milliseconds
};

const byVisitTimelineOptions = {
  timeAxis: { scale: "hour" },
  orientation: { axis: "top" },
  verticalScroll: true,
  xss: { disabled: true }, // Items are XSS safe through JQuery
  stack: false,
  stackSubgroups: false,
  zoomMin: 1000 * 60 * 60, // A single hour in milliseconds
  zoomMax: 1000 * 60 * 60 * 24, // A single day in milliseconds
};

/************************************ Initialize ************************************/

// Vehicle management state
let addingVehicleMode = false;
let pickingVehicleLocation = false;
let tempVehicleMarker = null;
let vehicleDeparturePicker = null;

// Route highlighting state
let highlightedVehicleId = null;
let routeNumberMarkers = [];  // Markers showing 1, 2, 3... on route stops


$(document).ready(function () {
  replaceQuickstartSolverForgeAutoHeaderFooter();

  // Initialize timelines after DOM is ready with a small delay to ensure Bootstrap tabs are rendered
  setTimeout(function () {
    const byVehiclePanel = document.getElementById("byVehiclePanel");
    const byVisitPanel = document.getElementById("byVisitPanel");

    if (byVehiclePanel) {
      byVehicleTimeline = new vis.Timeline(
        byVehiclePanel,
        byVehicleItemData,
        byVehicleGroupData,
        byVehicleTimelineOptions,
      );
    }

    if (byVisitPanel) {
      byVisitTimeline = new vis.Timeline(
        byVisitPanel,
        byVisitItemData,
        byVisitGroupData,
        byVisitTimelineOptions,
      );
    }
  }, 100);

  L.tileLayer("https://{s}.tile.openstreetmap.org/{z}/{x}/{y}.png", {
    maxZoom: 19,
    attribution:
      '&copy; <a href="https://www.openstreetmap.org/">OpenStreetMap</a> contributors',
  }).addTo(map);

  solveButton.click(solve);
  stopSolvingButton.click(stopSolving);
  analyzeButton.click(analyze);
  refreshSolvingButtons(false);

  // HACK to allow vis-timeline to work within Bootstrap tabs
  $("#byVehicleTab").on("shown.bs.tab", function (event) {
    if (byVehicleTimeline) {
      byVehicleTimeline.redraw();
    }
  });
  $("#byVisitTab").on("shown.bs.tab", function (event) {
    if (byVisitTimeline) {
      byVisitTimeline.redraw();
    }
  });

  // Map click handler - context aware
  map.on("click", function (e) {
    if (addingVehicleMode) {
      // Set vehicle home location
      setVehicleHomeLocation(e.latlng.lat, e.latlng.lng);
    } else if (!optimizing) {
      // Add new visit
      visitMarker = L.circleMarker(e.latlng);
      visitMarker.setStyle({ color: "green" });
      visitMarker.addTo(map);
      openRecommendationModal(e.latlng.lat, e.latlng.lng);
    }
  });

  // Remove visit marker when modal closes
  $("#newVisitModal").on("hidden.bs.modal", function () {
    if (visitMarker) {
      map.removeLayer(visitMarker);
    }
  });

  // Vehicle management
  $("#addVehicleBtn").click(openAddVehicleModal);
  $("#removeVehicleBtn").click(removeLastVehicle);
  $("#confirmAddVehicle").click(confirmAddVehicle);
  $("#pickLocationBtn").click(pickVehicleLocationOnMap);

  // Clean up when add vehicle modal closes (only if not picking location)
  $("#addVehicleModal").on("hidden.bs.modal", function () {
    if (!pickingVehicleLocation) {
      addingVehicleMode = false;
      if (tempVehicleMarker) {
        map.removeLayer(tempVehicleMarker);
        tempVehicleMarker = null;
      }
    }
  });

  // Real Roads toggle handler
  $(document).on('change', '#realRoadRouting', function() {
    useRealRoads = $(this).is(':checked');

    // If we have a demo dataset loaded, reload it with the new routing mode
    if (demoDataId && !optimizing) {
      scheduleId = null;
      initialized = false;
      homeLocationGroup.clearLayers();
      homeLocationMarkerByIdMap.clear();
      visitGroup.clearLayers();
      visitMarkerByIdMap.clear();
      routeGeometries = null;
      refreshRoutePlan();
    }
  });

  setupAjax();
  fetchDemoData();
});

/*************************************** Vehicle Management **************************************/

function openAddVehicleModal() {
  if (optimizing) {
    alert("Cannot add vehicles while solving. Please stop solving first.");
    return;
  }
  if (!loadedRoutePlan) {
    alert("Please load a dataset first.");
    return;
  }

  addingVehicleMode = true;

  // Suggest next vehicle name
  $("#vehicleName").val("").attr("placeholder", `e.g., ${getNextVehicleName()}`);

  // Set default values based on existing vehicles
  const existingVehicle = loadedRoutePlan.vehicles[0];
  if (existingVehicle) {
    $("#vehicleCapacity").val(existingVehicle.capacity || 25);
    const defaultLat = existingVehicle.homeLocation[0];
    const defaultLng = existingVehicle.homeLocation[1];
    $("#vehicleHomeLat").val(defaultLat.toFixed(6));
    $("#vehicleHomeLng").val(defaultLng.toFixed(6));
  }

  // Initialize departure time picker
  const tomorrow = JSJoda.LocalDate.now().plusDays(1);
  const defaultDeparture = tomorrow.atTime(JSJoda.LocalTime.of(6, 0));

  if (vehicleDeparturePicker) {
    vehicleDeparturePicker.destroy();
  }
  vehicleDeparturePicker = flatpickr("#vehicleDepartureTime", {
    enableTime: true,
    dateFormat: "Y-m-d H:i",
    defaultDate: defaultDeparture.format(JSJoda.DateTimeFormatter.ofPattern('yyyy-M-d HH:mm'))
  });

  $("#addVehicleModal").modal("show");
}

function pickVehicleLocationOnMap() {
  // Hide modal temporarily while user picks location
  pickingVehicleLocation = true;
  addingVehicleMode = true;
  $("#addVehicleModal").modal("hide");

  // Show hint on map
  $("#mapHint").html('<i class="fas fa-crosshairs"></i> Click on the map to set vehicle depot location').removeClass("hidden");
}

function setVehicleHomeLocation(lat, lng) {
  $("#vehicleHomeLat").val(lat.toFixed(6));
  $("#vehicleHomeLng").val(lng.toFixed(6));
  $("#vehicleLocationPreview").html(`<i class="fas fa-check text-success"></i> Location set: ${lat.toFixed(4)}, ${lng.toFixed(4)}`);

  // Show temporary marker
  if (tempVehicleMarker) {
    map.removeLayer(tempVehicleMarker);
  }
  tempVehicleMarker = L.marker([lat, lng], {
    icon: L.divIcon({
      className: 'temp-vehicle-marker',
      html: `<div style="
        background-color: #6366f1;
        border: 3px solid white;
        border-radius: 4px;
        width: 28px;
        height: 28px;
        display: flex;
        align-items: center;
        justify-content: center;
        box-shadow: 0 2px 4px rgba(0,0,0,0.4);
        animation: pulse 1s infinite;
      "><i class="fas fa-warehouse" style="color: white; font-size: 12px;"></i></div>`,
      iconSize: [28, 28],
      iconAnchor: [14, 14]
    })
  });
  tempVehicleMarker.addTo(map);

  // If we were picking location, re-open the modal
  if (pickingVehicleLocation) {
    pickingVehicleLocation = false;
    addingVehicleMode = false;
    $("#addVehicleModal").modal("show");
    // Restore normal map hint
    $("#mapHint").html('<i class="fas fa-mouse-pointer"></i> Click on the map to add a new visit');
  }
}

// Extended phonetic alphabet for generating vehicle names
const PHONETIC_NAMES = ["Alpha", "Bravo", "Charlie", "Delta", "Echo", "Foxtrot", "Golf", "Hotel", "India", "Juliet", "Kilo", "Lima", "Mike", "November", "Oscar", "Papa", "Quebec", "Romeo", "Sierra", "Tango", "Uniform", "Victor", "Whiskey", "X-ray", "Yankee", "Zulu"];

function getNextVehicleName() {
  if (!loadedRoutePlan) return "Alpha";
  const usedNames = new Set(loadedRoutePlan.vehicles.map(v => v.name));
  for (const name of PHONETIC_NAMES) {
    if (!usedNames.has(name)) return name;
  }
  // Fallback if all names used
  return `Vehicle ${loadedRoutePlan.vehicles.length + 1}`;
}

async function confirmAddVehicle() {
  const vehicleName = $("#vehicleName").val().trim() || getNextVehicleName();
  const capacity = parseInt($("#vehicleCapacity").val());
  const lat = parseFloat($("#vehicleHomeLat").val());
  const lng = parseFloat($("#vehicleHomeLng").val());
  const departureTime = $("#vehicleDepartureTime").val();

  if (!capacity || capacity < 1) {
    alert("Please enter a valid capacity (minimum 1).");
    return;
  }
  if (isNaN(lat) || isNaN(lng)) {
    alert("Please set a valid home location by clicking on the map or entering coordinates.");
    return;
  }
  if (!departureTime) {
    alert("Please set a departure time.");
    return;
  }

  // Generate new vehicle ID
  const maxId = Math.max(...loadedRoutePlan.vehicles.map(v => parseInt(v.id)), 0);
  const newId = String(maxId + 1);

  // Format departure time
  const formattedDeparture = JSJoda.LocalDateTime.parse(
    departureTime,
    JSJoda.DateTimeFormatter.ofPattern('yyyy-M-d HH:mm')
  ).format(JSJoda.DateTimeFormatter.ISO_LOCAL_DATE_TIME);

  // Create new vehicle
  const newVehicle = {
    id: newId,
    name: vehicleName,
    capacity: capacity,
    homeLocation: [lat, lng],
    departureTime: formattedDeparture,
    visits: [],
    totalDemand: 0,
    totalDrivingTimeSeconds: 0,
    arrivalTime: formattedDeparture
  };

  // Add to solution
  loadedRoutePlan.vehicles.push(newVehicle);

  // Close modal and refresh
  $("#addVehicleModal").modal("hide");
  addingVehicleMode = false;

  if (tempVehicleMarker) {
    map.removeLayer(tempVehicleMarker);
    tempVehicleMarker = null;
  }

  // Refresh display
  await renderRoutes(loadedRoutePlan);
  renderTimelines(loadedRoutePlan);

  showNotification(`Vehicle "${vehicleName}" added successfully!`, "success");
}

async function removeLastVehicle() {
  if (optimizing) {
    alert("Cannot remove vehicles while solving. Please stop solving first.");
    return;
  }
  if (!loadedRoutePlan || loadedRoutePlan.vehicles.length <= 1) {
    alert("Cannot remove the last vehicle. At least one vehicle is required.");
    return;
  }

  const lastVehicle = loadedRoutePlan.vehicles[loadedRoutePlan.vehicles.length - 1];

  if (lastVehicle.visits && lastVehicle.visits.length > 0) {
    if (!confirm(`Vehicle ${lastVehicle.id} has ${lastVehicle.visits.length} assigned visits. These will become unassigned. Continue?`)) {
      return;
    }
    // Unassign visits from the vehicle
    lastVehicle.visits.forEach(visitId => {
      const visit = loadedRoutePlan.visits.find(v => v.id === visitId);
      if (visit) {
        visit.vehicle = null;
        visit.previousVisit = null;
        visit.nextVisit = null;
        visit.arrivalTime = null;
        visit.departureTime = null;
      }
    });
  }

  // Remove vehicle
  loadedRoutePlan.vehicles.pop();

  // Remove marker
  const marker = homeLocationMarkerByIdMap.get(lastVehicle.id);
  if (marker) {
    homeLocationGroup.removeLayer(marker);
    homeLocationMarkerByIdMap.delete(lastVehicle.id);
  }

  // Refresh display
  await renderRoutes(loadedRoutePlan);
  renderTimelines(loadedRoutePlan);

  showNotification(`Vehicle "${lastVehicle.name || lastVehicle.id}" removed.`, "info");
}

async function removeVehicle(vehicleId) {
  if (optimizing) {
    alert("Cannot remove vehicles while solving. Please stop solving first.");
    return;
  }

  const vehicleIndex = loadedRoutePlan.vehicles.findIndex(v => v.id === vehicleId);
  if (vehicleIndex === -1) return;

  if (loadedRoutePlan.vehicles.length <= 1) {
    alert("Cannot remove the last vehicle. At least one vehicle is required.");
    return;
  }

  const vehicle = loadedRoutePlan.vehicles[vehicleIndex];

  if (vehicle.visits && vehicle.visits.length > 0) {
    if (!confirm(`Vehicle ${vehicle.id} has ${vehicle.visits.length} assigned visits. These will become unassigned. Continue?`)) {
      return;
    }
    // Unassign visits
    vehicle.visits.forEach(visitId => {
      const visit = loadedRoutePlan.visits.find(v => v.id === visitId);
      if (visit) {
        visit.vehicle = null;
        visit.previousVisit = null;
        visit.nextVisit = null;
        visit.arrivalTime = null;
        visit.departureTime = null;
      }
    });
  }

  // Remove vehicle
  loadedRoutePlan.vehicles.splice(vehicleIndex, 1);

  // Remove marker
  const marker = homeLocationMarkerByIdMap.get(vehicleId);
  if (marker) {
    homeLocationGroup.removeLayer(marker);
    homeLocationMarkerByIdMap.delete(vehicleId);
  }

  // Refresh display
  await renderRoutes(loadedRoutePlan);
  renderTimelines(loadedRoutePlan);

  showNotification(`Vehicle "${vehicle.name || vehicleId}" removed.`, "info");
}

function showNotification(message, type = "info") {
  const alertClass = type === "success" ? "alert-success" : type === "error" ? "alert-danger" : "alert-info";
  const icon = type === "success" ? "fa-check-circle" : type === "error" ? "fa-exclamation-circle" : "fa-info-circle";

  const notification = $(`
    <div class="alert ${alertClass} alert-dismissible fade show" role="alert" style="min-width: 300px;">
      <i class="fas ${icon} me-2"></i>${message}
      <button type="button" class="btn-close" data-bs-dismiss="alert" aria-label="Close"></button>
    </div>
  `);

  $("#notificationPanel").append(notification);

  // Auto-dismiss after 3 seconds
  setTimeout(() => {
    notification.alert('close');
  }, 3000);
}

/*************************************** Route Highlighting **************************************/

function toggleVehicleHighlight(vehicleId) {
  if (highlightedVehicleId === vehicleId) {
    // Already highlighted - clear it
    clearRouteHighlight();
  } else {
    // Highlight this vehicle's route
    highlightVehicleRoute(vehicleId);
  }
}

function clearRouteHighlight() {
  // Remove number markers
  routeNumberMarkers.forEach(marker => map.removeLayer(marker));
  routeNumberMarkers = [];

  // Reset all vehicle icons to normal and restore opacity
  if (loadedRoutePlan) {
    loadedRoutePlan.vehicles.forEach(vehicle => {
      const marker = homeLocationMarkerByIdMap.get(vehicle.id);
      if (marker) {
        marker.setIcon(createVehicleHomeIcon(vehicle, false));
        marker.setOpacity(1);
      }
    });

    // Reset all visit markers to normal and restore opacity
    loadedRoutePlan.visits.forEach(visit => {
      const marker = visitMarkerByIdMap.get(visit.id);
      if (marker) {
        const customerType = getCustomerType(visit);
        const isAssigned = visit.vehicle != null;
        marker.setIcon(createCustomerTypeIcon(customerType, isAssigned, false));
        marker.setOpacity(1);
      }
    });
  }

  // Reset route lines
  renderRouteLines();

  // Update vehicle table highlighting
  $("#vehicles tr").removeClass("table-active");

  highlightedVehicleId = null;
}

function highlightVehicleRoute(vehicleId) {
  // Clear any existing highlight first
  clearRouteHighlight();

  highlightedVehicleId = vehicleId;

  if (!loadedRoutePlan) return;

  const vehicle = loadedRoutePlan.vehicles.find(v => v.id === vehicleId);
  if (!vehicle) return;

  const vehicleColor = colorByVehicle(vehicle);

  // Highlight the vehicle's home marker
  const homeMarker = homeLocationMarkerByIdMap.get(vehicleId);
  if (homeMarker) {
    homeMarker.setIcon(createVehicleHomeIcon(vehicle, true));
  }

  // Dim other vehicles
  loadedRoutePlan.vehicles.forEach(v => {
    if (v.id !== vehicleId) {
      const marker = homeLocationMarkerByIdMap.get(v.id);
      if (marker) {
        marker.setIcon(createVehicleHomeIcon(v, false));
        marker.setOpacity(0.3);
      }
    }
  });

  // Get visit order for this vehicle
  const visitByIdMap = new Map(loadedRoutePlan.visits.map(v => [v.id, v]));
  const vehicleVisits = vehicle.visits.map(visitId => visitByIdMap.get(visitId)).filter(v => v);

  // Highlight and number the visits on this route
  let stopNumber = 1;
  vehicleVisits.forEach(visit => {
    const marker = visitMarkerByIdMap.get(visit.id);
    if (marker) {
      const customerType = getCustomerType(visit);
      marker.setIcon(createCustomerTypeIcon(customerType, true, true, vehicleColor));
      marker.setOpacity(1);

      // Add number marker
      const numberMarker = L.marker(visit.location, {
        icon: createRouteNumberIcon(stopNumber, vehicleColor),
        interactive: false,
        zIndexOffset: 1000
      });
      numberMarker.addTo(map);
      routeNumberMarkers.push(numberMarker);
      stopNumber++;
    }
  });

  // Dim visits not on this route
  loadedRoutePlan.visits.forEach(visit => {
    if (!vehicle.visits.includes(visit.id)) {
      const marker = visitMarkerByIdMap.get(visit.id);
      if (marker) {
        marker.setOpacity(0.25);
      }
    }
  });

  // Highlight just this route, dim others
  renderRouteLines(vehicleId);

  // Highlight the row in the vehicle table
  $("#vehicles tr").removeClass("table-active");
  $(`#vehicle-row-${vehicleId}`).addClass("table-active");

  // Add start marker (S) at depot
  const startMarker = L.marker(vehicle.homeLocation, {
    icon: createRouteNumberIcon("S", vehicleColor),
    interactive: false,
    zIndexOffset: 1000
  });
  startMarker.addTo(map);
  routeNumberMarkers.push(startMarker);
}

function createRouteNumberIcon(number, color) {
  return L.divIcon({
    className: 'route-number-marker',
    html: `<div style="
      background-color: ${color};
      color: white;
      font-weight: bold;
      font-size: 12px;
      width: 22px;
      height: 22px;
      border-radius: 50%;
      border: 2px solid white;
      display: flex;
      align-items: center;
      justify-content: center;
      box-shadow: 0 2px 4px rgba(0,0,0,0.4);
      margin-left: 16px;
      margin-top: -28px;
    ">${number}</div>`,
    iconSize: [22, 22],
    iconAnchor: [0, 0]
  });
}

async function renderRouteLines(highlightedId = null) {
  routeGroup.clearLayers();

  if (!loadedRoutePlan) return;

  // Fetch geometries during solving (routes change)
  if (scheduleId) {
    routeGeometries = await fetchRouteGeometries();
  }

  const visitByIdMap = new Map(loadedRoutePlan.visits.map(visit => [visit.id, visit]));

  for (let vehicle of loadedRoutePlan.vehicles) {
    const homeLocation = vehicle.homeLocation;
    const locations = vehicle.visits.map(visitId => visitByIdMap.get(visitId)?.location).filter(l => l);

    const isHighlighted = highlightedId === null || vehicle.id === highlightedId;
    const color = colorByVehicle(vehicle);
    const weight = isHighlighted && highlightedId !== null ? 5 : 3;
    const opacity = isHighlighted ? 1 : 0.2;

    const vehicleGeometry = routeGeometries?.[vehicle.id];

    if (vehicleGeometry && vehicleGeometry.length > 0) {
      // Draw real road routes using decoded polylines
      for (const encodedSegment of vehicleGeometry) {
        if (encodedSegment) {
          const points = decodePolyline(encodedSegment);
          if (points.length > 0) {
            L.polyline(points, {
              color: color,
              weight: weight,
              opacity: opacity
            }).addTo(routeGroup);
          }
        }
      }
    } else if (locations.length > 0) {
      // Fallback to straight lines if no geometry available
      L.polyline([homeLocation, ...locations, homeLocation], {
        color: color,
        weight: weight,
        opacity: opacity
      }).addTo(routeGroup);
    }
  }
}

function colorByVehicle(vehicle) {
  return vehicle === null ? null : pickColor("vehicle" + vehicle.id);
}

// Customer type definitions matching demo_data.py
const CUSTOMER_TYPES = {
  RESTAURANT: { label: "Restaurant", icon: "fa-utensils", color: "#f59e0b", windowStart: "06:00", windowEnd: "10:00", minService: 20, maxService: 40 },
  BUSINESS: { label: "Business", icon: "fa-building", color: "#3b82f6", windowStart: "09:00", windowEnd: "17:00", minService: 15, maxService: 30 },
  RESIDENTIAL: { label: "Residential", icon: "fa-home", color: "#10b981", windowStart: "17:00", windowEnd: "20:00", minService: 5, maxService: 10 },
};

function getCustomerType(visit) {
  const startTime = showTimeOnly(visit.minStartTime).toString();
  const endTime = showTimeOnly(visit.maxEndTime).toString();

  for (const [type, config] of Object.entries(CUSTOMER_TYPES)) {
    if (startTime === config.windowStart && endTime === config.windowEnd) {
      return { type, ...config };
    }
  }
  return { type: "UNKNOWN", label: "Custom", icon: "fa-question", color: "#6b7280", windowStart: startTime, windowEnd: endTime };
}

function formatDrivingTime(drivingTimeInSeconds) {
  return `${Math.floor(drivingTimeInSeconds / 3600)}h ${Math.round((drivingTimeInSeconds % 3600) / 60)}m`;
}

function homeLocationPopupContent(vehicle) {
  const color = colorByVehicle(vehicle);
  const visitCount = vehicle.visits ? vehicle.visits.length : 0;
  const vehicleName = vehicle.name || `Vehicle ${vehicle.id}`;
  return `<div style="min-width: 150px;">
    <h5 style="color: ${color};"><i class="fas fa-truck"></i> ${vehicleName}</h5>
    <p class="mb-1"><strong>Depot Location</strong></p>
    <p class="mb-1"><i class="fas fa-box"></i> Capacity: ${vehicle.capacity}</p>
    <p class="mb-1"><i class="fas fa-route"></i> Visits: ${visitCount}</p>
    <p class="mb-0"><i class="fas fa-clock"></i> Departs: ${showTimeOnly(vehicle.departureTime)}</p>
  </div>`;
}

function visitPopupContent(visit) {
  const customerType = getCustomerType(visit);
  const serviceDurationMinutes = Math.round(visit.serviceDuration / 60);
  const arrival = visit.arrivalTime
    ? `<h6>Arrival at ${showTimeOnly(visit.arrivalTime)}.</h6>`
    : "";
  return `<h5><i class="fas ${customerType.icon}" style="color: ${customerType.color}"></i> ${visit.name}</h5>
    <h6><span class="badge" style="background-color: ${customerType.color}">${customerType.label}</span></h6>
    <h6>Cargo: ${visit.demand} units</h6>
    <h6>Service time: ${serviceDurationMinutes} min</h6>
    <h6>Window: ${showTimeOnly(visit.minStartTime)} - ${showTimeOnly(visit.maxEndTime)}</h6>
    ${arrival}`;
}

function showTimeOnly(localDateTimeString) {
  return JSJoda.LocalDateTime.parse(localDateTimeString).toLocalTime();
}

function createVehicleHomeIcon(vehicle, isHighlighted = false) {
  const color = colorByVehicle(vehicle);
  const size = isHighlighted ? 36 : 28;
  const fontSize = isHighlighted ? 14 : 11;
  const borderWidth = isHighlighted ? 4 : 3;
  const shadow = isHighlighted
    ? `0 0 0 4px ${color}40, 0 4px 8px rgba(0,0,0,0.5)`
    : '0 2px 4px rgba(0,0,0,0.4)';

  return L.divIcon({
    className: 'vehicle-home-marker',
    html: `<div style="
      background-color: ${color};
      border: ${borderWidth}px solid white;
      border-radius: 50%;
      width: ${size}px;
      height: ${size}px;
      display: flex;
      align-items: center;
      justify-content: center;
      box-shadow: ${shadow};
      transition: all 0.2s ease;
    "><i class="fas fa-truck" style="color: white; font-size: ${fontSize}px;"></i></div>`,
    iconSize: [size, size],
    iconAnchor: [size/2, size/2],
    popupAnchor: [0, -size/2]
  });
}

function getHomeLocationMarker(vehicle) {
  let marker = homeLocationMarkerByIdMap.get(vehicle.id);
  if (marker) {
    marker.setIcon(createVehicleHomeIcon(vehicle));
    return marker;
  }
  marker = L.marker(vehicle.homeLocation, {
    icon: createVehicleHomeIcon(vehicle)
  });
  marker.addTo(homeLocationGroup).bindPopup();
  homeLocationMarkerByIdMap.set(vehicle.id, marker);
  return marker;
}

function createCustomerTypeIcon(customerType, isAssigned = false, isHighlighted = false, highlightColor = null) {
  const borderColor = isHighlighted && highlightColor
    ? highlightColor
    : (isAssigned ? customerType.color : '#6b7280');
  const size = isHighlighted ? 38 : 32;
  const fontSize = isHighlighted ? 16 : 14;
  const borderWidth = isHighlighted ? 4 : 3;
  const shadow = isHighlighted
    ? `0 0 0 4px ${highlightColor}40, 0 4px 8px rgba(0,0,0,0.4)`
    : '0 2px 4px rgba(0,0,0,0.3)';

  return L.divIcon({
    className: 'customer-marker',
    html: `<div style="
      background-color: white;
      border: ${borderWidth}px solid ${borderColor};
      border-radius: 50%;
      width: ${size}px;
      height: ${size}px;
      display: flex;
      align-items: center;
      justify-content: center;
      box-shadow: ${shadow};
      transition: all 0.2s ease;
    "><i class="fas ${customerType.icon}" style="color: ${customerType.color}; font-size: ${fontSize}px;"></i></div>`,
    iconSize: [size, size],
    iconAnchor: [size/2, size/2],
    popupAnchor: [0, -size/2]
  });
}

function getVisitMarker(visit) {
  let marker = visitMarkerByIdMap.get(visit.id);
  const customerType = getCustomerType(visit);
  const isAssigned = visit.vehicle != null;

  if (marker) {
    // Update icon if assignment status changed
    marker.setIcon(createCustomerTypeIcon(customerType, isAssigned));
    return marker;
  }

  marker = L.marker(visit.location, {
    icon: createCustomerTypeIcon(customerType, isAssigned)
  });
  marker.addTo(visitGroup).bindPopup();
  visitMarkerByIdMap.set(visit.id, marker);
  return marker;
}

async function renderRoutes(solution) {
  if (!initialized) {
    const bounds = [solution.southWestCorner, solution.northEastCorner];
    map.fitBounds(bounds);
  }
  // Vehicles
  vehiclesTable.children().remove();
  const canRemove = solution.vehicles.length > 1;
  solution.vehicles.forEach(function (vehicle) {
    getHomeLocationMarker(vehicle).setPopupContent(
      homeLocationPopupContent(vehicle),
    );
    const { id, capacity, totalDemand, totalDrivingTimeSeconds } = vehicle;
    const percentage = Math.min((totalDemand / capacity) * 100, 100);
    const overCapacity = totalDemand > capacity;
    const color = colorByVehicle(vehicle);
    const progressBarColor = overCapacity ? 'bg-danger' : '';
    const isHighlighted = highlightedVehicleId === id;
    const visitCount = vehicle.visits ? vehicle.visits.length : 0;
    const vehicleName = vehicle.name || `Vehicle ${id}`;

    vehiclesTable.append(`
      <tr id="vehicle-row-${id}" class="vehicle-row ${isHighlighted ? 'table-active' : ''}" style="cursor: pointer;">
        <td onclick="toggleVehicleHighlight('${id}')">
          <div style="background-color: ${color}; width: 1.5rem; height: 1.5rem; border-radius: 50%; display: flex; align-items: center; justify-content: center; ${isHighlighted ? 'box-shadow: 0 0 0 3px ' + color + '40;' : ''}">
            <i class="fas fa-truck" style="color: white; font-size: 0.65rem;"></i>
          </div>
        </td>
        <td onclick="toggleVehicleHighlight('${id}')">
          <strong>${vehicleName}</strong>
          <br><small class="text-muted">${visitCount} stops</small>
        </td>
        <td onclick="toggleVehicleHighlight('${id}')">
          <div class="progress" style="height: 18px;" data-bs-toggle="tooltip" data-bs-placement="left"
            title="Cargo: ${totalDemand} / Capacity: ${capacity}${overCapacity ? ' (OVER CAPACITY!)' : ''}">
            <div class="progress-bar ${progressBarColor}" role="progressbar" style="width: ${percentage}%; font-size: 0.7rem; transition: width 0.3s ease;">
              ${totalDemand}/${capacity}
            </div>
          </div>
        </td>
        <td onclick="toggleVehicleHighlight('${id}')" style="font-size: 0.85rem;">
          ${formatDrivingTime(totalDrivingTimeSeconds)}
        </td>
        <td>
          ${canRemove ? `<button class="btn btn-sm btn-outline-danger p-0 px-1" onclick="event.stopPropagation(); removeVehicle('${id}')" title="Remove vehicle ${vehicleName}">
            <i class="fas fa-times" style="font-size: 0.7rem;"></i>
          </button>` : ''}
        </td>
      </tr>`);
  });
  // Visits
  solution.visits.forEach(function (visit) {
    getVisitMarker(visit).setPopupContent(visitPopupContent(visit));
  });
  // Route - use the dedicated function which handles highlighting (await to ensure geometries load)
  await renderRouteLines(highlightedVehicleId);

  // Summary
  $("#score").text(solution.score ? `Score: ${solution.score}` : "Score: ?");
  $("#drivingTime").text(formatDrivingTime(solution.totalDrivingTimeSeconds));
}

function renderTimelines(routePlan) {
  byVehicleGroupData.clear();
  byVisitGroupData.clear();
  byVehicleItemData.clear();
  byVisitItemData.clear();

  // Build lookup maps for O(1) access
  const vehicleById = new Map(routePlan.vehicles.map(v => [v.id, v]));
  const visitById = new Map(routePlan.visits.map(v => [v.id, v]));
  const visitOrderMap = new Map();

  // Build stop order for each visit
  routePlan.vehicles.forEach(vehicle => {
    vehicle.visits.forEach((visitId, index) => {
      visitOrderMap.set(visitId, index + 1);
    });
  });

  // Vehicle groups with names and status summary
  $.each(routePlan.vehicles, function (index, vehicle) {
    const vehicleName = vehicle.name || `Vehicle ${vehicle.id}`;
    const { totalDemand, capacity } = vehicle;
    const percentage = Math.min((totalDemand / capacity) * 100, 100);
    const overCapacity = totalDemand > capacity;

    // Count late visits for this vehicle
    const vehicleVisits = vehicle.visits.map(id => visitById.get(id)).filter(v => v);
    const lateCount = vehicleVisits.filter(v => {
      if (!v.departureTime) return false;
      const departure = JSJoda.LocalDateTime.parse(v.departureTime);
      const maxEnd = JSJoda.LocalDateTime.parse(v.maxEndTime);
      return departure.isAfter(maxEnd);
    }).length;

    const statusIcon = lateCount > 0
      ? `<i class="fas fa-exclamation-triangle timeline-status-late timeline-status-icon" title="${lateCount} late"></i>`
      : vehicle.visits.length > 0
        ? `<i class="fas fa-check-circle timeline-status-ontime timeline-status-icon" title="All on-time"></i>`
        : '';

    const progressBarClass = overCapacity ? 'bg-danger' : '';

    const vehicleWithLoad = `
      <h5 class="card-title mb-1">${vehicleName}${statusIcon}</h5>
      <div class="progress" style="height: 16px;" title="Cargo: ${totalDemand} / ${capacity}">
        <div class="progress-bar ${progressBarClass}" role="progressbar" style="width: ${percentage}%">
          ${totalDemand}/${capacity}
        </div>
      </div>`;
    byVehicleGroupData.add({ id: vehicle.id, content: vehicleWithLoad });
  });

  $.each(routePlan.visits, function (index, visit) {
    const minStartTime = JSJoda.LocalDateTime.parse(visit.minStartTime);
    const maxEndTime = JSJoda.LocalDateTime.parse(visit.maxEndTime);
    const serviceDuration = JSJoda.Duration.ofSeconds(visit.serviceDuration);
    const customerType = getCustomerType(visit);
    const stopNumber = visitOrderMap.get(visit.id);

    const visitGroupElement = $(`<div/>`).append(
      $(`<h5 class="card-title mb-1"/>`).html(
        `<i class="fas ${customerType.icon}" style="color: ${customerType.color}"></i> ${visit.name}`
      ),
    ).append(
      $(`<small class="text-muted"/>`).text(customerType.label)
    );
    byVisitGroupData.add({
      id: visit.id,
      content: visitGroupElement.html(),
    });

    // Time window per visit.
    byVisitItemData.add({
      id: visit.id + "_readyToDue",
      group: visit.id,
      start: visit.minStartTime,
      end: visit.maxEndTime,
      type: "background",
      style: "background-color: #8AE23433",
    });

    if (visit.vehicle == null) {
      const byJobJobElement = $(`<div/>`).append(
        $(`<span/>`).html(`<i class="fas fa-exclamation-circle text-danger me-1"></i>Unassigned`),
      );

      // Unassigned are shown at the beginning of the visit's time window; the length is the service duration.
      byVisitItemData.add({
        id: visit.id + "_unassigned",
        group: visit.id,
        content: byJobJobElement.html(),
        start: minStartTime.toString(),
        end: minStartTime.plus(serviceDuration).toString(),
        style: "background-color: #EF292999",
      });
    } else {
      const arrivalTime = JSJoda.LocalDateTime.parse(visit.arrivalTime);
      const beforeReady = arrivalTime.isBefore(minStartTime);
      const departureTime = JSJoda.LocalDateTime.parse(visit.departureTime);
      const afterDue = departureTime.isAfter(maxEndTime);

      // Get vehicle info for display
      const vehicleInfo = vehicleById.get(visit.vehicle);
      const vehicleName = vehicleInfo ? (vehicleInfo.name || `Vehicle ${visit.vehicle}`) : `Vehicle ${visit.vehicle}`;

      // Stop badge for service segment
      const stopBadge = stopNumber ? `<span class="timeline-stop-badge">${stopNumber}</span>` : '';

      // Status icon based on timing
      const statusIcon = afterDue
        ? `<i class="fas fa-exclamation-triangle timeline-status-late timeline-status-icon" title="Late"></i>`
        : `<i class="fas fa-check timeline-status-ontime timeline-status-icon" title="On-time"></i>`;

      const byVehicleElement = $(`<div/>`)
        .append($(`<span/>`).html(
          `${stopBadge}<i class="fas ${customerType.icon}" style="color: ${customerType.color}"></i> ${visit.name}${statusIcon}`
        ));

      const byVisitElement = $(`<div/>`)
        .append(
          $(`<span/>`).html(
            `${stopBadge}${vehicleName}${statusIcon}`
          ),
        );

      const byVehicleTravelElement = $(`<div/>`).append(
        $(`<span/>`).html(`<i class="fas fa-route text-warning me-1"></i>Travel`),
      );

      const previousDeparture = arrivalTime.minusSeconds(
        visit.drivingTimeSecondsFromPreviousStandstill,
      );
      byVehicleItemData.add({
        id: visit.id + "_travel",
        group: visit.vehicle,
        subgroup: visit.vehicle,
        content: byVehicleTravelElement.html(),
        start: previousDeparture.toString(),
        end: visit.arrivalTime,
        style: "background-color: #f7dd8f90",
      });

      if (beforeReady) {
        const byVehicleWaitElement = $(`<div/>`).append(
          $(`<span/>`).html(`<i class="fas fa-clock timeline-status-early me-1"></i>Wait`),
        );

        byVehicleItemData.add({
          id: visit.id + "_wait",
          group: visit.vehicle,
          subgroup: visit.vehicle,
          content: byVehicleWaitElement.html(),
          start: visit.arrivalTime,
          end: visit.minStartTime,
          style: "background-color: #93c5fd80",
        });
      }

      let serviceElementBackground = afterDue ? "#EF292999" : "#83C15955";

      byVehicleItemData.add({
        id: visit.id + "_service",
        group: visit.vehicle,
        subgroup: visit.vehicle,
        content: byVehicleElement.html(),
        start: visit.startServiceTime,
        end: visit.departureTime,
        style: "background-color: " + serviceElementBackground,
      });
      byVisitItemData.add({
        id: visit.id,
        group: visit.id,
        content: byVisitElement.html(),
        start: visit.startServiceTime,
        end: visit.departureTime,
        style: "background-color: " + serviceElementBackground,
      });
    }
  });

  $.each(routePlan.vehicles, function (index, vehicle) {
    if (vehicle.visits.length > 0) {
      let lastVisit = routePlan.visits
        .filter(
          (visit) => visit.id == vehicle.visits[vehicle.visits.length - 1],
        )
        .pop();
      if (lastVisit) {
        byVehicleItemData.add({
          id: vehicle.id + "_travelBackToHomeLocation",
          group: vehicle.id,
          subgroup: vehicle.id,
          content: $(`<div/>`)
            .append($(`<span/>`).html(`<i class="fas fa-home text-secondary me-1"></i>Return`))
            .html(),
          start: lastVisit.departureTime,
          end: vehicle.arrivalTime,
          style: "background-color: #f7dd8f90",
        });
      }
    }
  });

  if (!initialized) {
    if (byVehicleTimeline) {
      byVehicleTimeline.setWindow(
        routePlan.startDateTime,
        routePlan.endDateTime,
      );
    }
    if (byVisitTimeline) {
      byVisitTimeline.setWindow(routePlan.startDateTime, routePlan.endDateTime);
    }
  }
}

function analyze() {
  // see score-analysis.js
  analyzeScore(loadedRoutePlan, "/route-plans/analyze");
}

function openRecommendationModal(lat, lng) {
  if (!('score' in loadedRoutePlan) || optimizing) {
    map.removeLayer(visitMarker);
    visitMarker = null;
    let message = "Please click the Solve button before adding new visits.";
    if (optimizing) {
      message = "Please wait for the solving process to finish.";
    }
    alert(message);
    return;
  }
  // see recommended-fit.js
  const visitId = Math.max(...loadedRoutePlan.visits.map(c => parseInt(c.id))) + 1;
  newVisit = {id: visitId, location: [lat, lng]};
  addNewVisit(visitId, lat, lng, map, visitMarker);
}

function getRecommendationsModal() {
  let formValid = true;
  formValid = validateFormField(newVisit, 'name', '#inputName') && formValid;
  formValid = validateFormField(newVisit, 'demand', '#inputDemand') && formValid;
  formValid = validateFormField(newVisit, 'minStartTime', '#inputMinStartTime') && formValid;
  formValid = validateFormField(newVisit, 'maxEndTime', '#inputMaxStartTime') && formValid;
  formValid = validateFormField(newVisit, 'serviceDuration', '#inputDuration') && formValid;

  if (formValid) {
    const updatedMinStartTime = JSJoda.LocalDateTime.parse(
      newVisit['minStartTime'],
      JSJoda.DateTimeFormatter.ofPattern('yyyy-M-d HH:mm')
    ).format(JSJoda.DateTimeFormatter.ISO_LOCAL_DATE_TIME);

    const updatedMaxEndTime = JSJoda.LocalDateTime.parse(
      newVisit['maxEndTime'],
      JSJoda.DateTimeFormatter.ofPattern('yyyy-M-d HH:mm')
    ).format(JSJoda.DateTimeFormatter.ISO_LOCAL_DATE_TIME);

    const updatedVisit = {
      ...newVisit,
      serviceDuration: parseInt(newVisit['serviceDuration']) * 60, // Convert minutes to seconds
      minStartTime: updatedMinStartTime,
      maxEndTime: updatedMaxEndTime
    };

    let updatedVisitList = [...loadedRoutePlan['visits']];
    updatedVisitList.push(updatedVisit);
    let updatedSolution = {...loadedRoutePlan, visits: updatedVisitList};

    // see recommended-fit.js
    requestRecommendations(updatedVisit.id, updatedSolution, "/route-plans/recommendation");
  }
}

function validateFormField(target, fieldName, inputName) {
  target[fieldName] = $(inputName).val();
  if ($(inputName).val() == "") {
    $(inputName).addClass("is-invalid");
  } else {
    $(inputName).removeClass("is-invalid");
  }
  return $(inputName).val() != "";
}

function applyRecommendationModal(recommendations) {
  let checkedRecommendation = null;
  recommendations.forEach((recommendation, index) => {
    if ($('#option' + index).is(":checked")) {
      checkedRecommendation = recommendations[index];
    }
  });

  if (!checkedRecommendation) {
    alert("Please select a recommendation.");
    return;
  }

  const updatedMinStartTime = JSJoda.LocalDateTime.parse(
    newVisit['minStartTime'],
    JSJoda.DateTimeFormatter.ofPattern('yyyy-M-d HH:mm')
  ).format(JSJoda.DateTimeFormatter.ISO_LOCAL_DATE_TIME);

  const updatedMaxEndTime = JSJoda.LocalDateTime.parse(
    newVisit['maxEndTime'],
    JSJoda.DateTimeFormatter.ofPattern('yyyy-M-d HH:mm')
  ).format(JSJoda.DateTimeFormatter.ISO_LOCAL_DATE_TIME);

  const updatedVisit = {
    ...newVisit,
    serviceDuration: parseInt(newVisit['serviceDuration']) * 60, // Convert minutes to seconds
    minStartTime: updatedMinStartTime,
    maxEndTime: updatedMaxEndTime
  };

  let updatedVisitList = [...loadedRoutePlan['visits']];
  updatedVisitList.push(updatedVisit);
  let updatedSolution = {...loadedRoutePlan, visits: updatedVisitList};

  // see recommended-fit.js
  applyRecommendation(
    updatedSolution,
    newVisit.id,
    checkedRecommendation.proposition.vehicleId,
    checkedRecommendation.proposition.index,
    "/route-plans/recommendation/apply"
  );
}

async function updateSolutionWithNewVisit(newSolution) {
  loadedRoutePlan = newSolution;
  await renderRoutes(newSolution);
  renderTimelines(newSolution);
  $('#newVisitModal').modal('hide');
}

// TODO: move the general functionality to the webjar.

function setupAjax() {
  $.ajaxSetup({
    headers: {
      "Content-Type": "application/json",
      Accept: "application/json,text/plain", // plain text is required by solve() returning UUID of the solver job
    },
  });

  // Extend jQuery to support $.put() and $.delete()
  jQuery.each(["put", "delete"], function (i, method) {
    jQuery[method] = function (url, data, callback, type) {
      if (jQuery.isFunction(data)) {
        type = type || callback;
        callback = data;
        data = undefined;
      }
      return jQuery.ajax({
        url: url,
        type: method,
        dataType: type,
        data: data,
        success: callback,
      });
    };
  });
}

function solve() {
  // Clear geometry cache - will be refreshed when solution updates
  routeGeometries = null;

  // Disable button immediately to prevent double-clicks
  $("#solveButton").prop("disabled", true).addClass("disabled");

  $.ajax({
    url: "/route-plans",
    type: "POST",
    data: JSON.stringify(loadedRoutePlan),
    contentType: "application/json",
    dataType: "text",
    success: function (data) {
      scheduleId = data.replace(/"/g, ""); // Remove quotes from UUID
      $("#solveButton").prop("disabled", false).removeClass("disabled");
      refreshSolvingButtons(true);
    },
    error: function (xhr, ajaxOptions, thrownError) {
      showError("Start solving failed.", xhr);
      $("#solveButton").prop("disabled", false).removeClass("disabled");
      refreshSolvingButtons(false);
    },
  });
}

function refreshSolvingButtons(solving) {
  optimizing = solving;
  if (solving) {
    $("#solveButton").hide();
    $("#visitButton").hide();
    $("#stopSolvingButton").show();
    $("#solvingSpinner").addClass("active");
    $("#mapHint").addClass("hidden");
    if (autoRefreshIntervalId == null) {
      autoRefreshIntervalId = setInterval(refreshRoutePlan, 2000);
    }
  } else {
    $("#solveButton").show();
    $("#visitButton").show();
    $("#stopSolvingButton").hide();
    $("#solvingSpinner").removeClass("active");
    $("#mapHint").removeClass("hidden");
    if (autoRefreshIntervalId != null) {
      clearInterval(autoRefreshIntervalId);
      autoRefreshIntervalId = null;
    }
  }
}

async function refreshRoutePlan() {
  let path = "/route-plans/" + scheduleId;
  let isLoadingDemoData = scheduleId === null;

  if (isLoadingDemoData) {
    if (demoDataId === null) {
      alert("Please select a test data set.");
      return;
    }

    // Clear geometry cache when loading new demo data
    routeGeometries = null;

    try {
      let routePlan;
      if (useRealRoads) {
        // Use SSE streaming for real roads to show progress
        routePlan = await loadDemoDataWithProgress(demoDataId);
      } else {
        // Use simple GET for haversine (instant, no loading overlay)
        routePlan = await $.getJSON(`/demo-data/${demoDataId}`);
      }
      loadedRoutePlan = routePlan;
      refreshSolvingButtons(
        routePlan.solverStatus != null &&
          routePlan.solverStatus !== "NOT_SOLVING",
      );
      await renderRoutes(routePlan);
      renderTimelines(routePlan);
      initialized = true;
    } catch (error) {
      showError("Getting demo data has failed: " + error.message, {});
      refreshSolvingButtons(false);
    }
    return;
  }

  // Loading existing route plan (during solving)
  try {
    const routePlan = await $.getJSON(path);
    loadedRoutePlan = routePlan;
    refreshSolvingButtons(
      routePlan.solverStatus != null &&
        routePlan.solverStatus !== "NOT_SOLVING",
    );
    await renderRoutes(routePlan);
    renderTimelines(routePlan);
    initialized = true;
  } catch (error) {
    showError("Getting route plan has failed.", error);
    refreshSolvingButtons(false);
  }
}

function stopSolving() {
  $.delete("/route-plans/" + scheduleId, function () {
    refreshSolvingButtons(false);
    refreshRoutePlan();
  }).fail(function (xhr, ajaxOptions, thrownError) {
    showError("Stop solving failed.", xhr);
  });
}

function fetchDemoData() {
  $.get("/demo-data", function (data) {
    data.forEach(function (item) {
      $("#testDataButton").append(
        $(
          '<a id="' +
            item +
            'TestData" class="dropdown-item" href="#">' +
            item +
            "</a>",
        ),
      );

      $("#" + item + "TestData").click(function () {
        switchDataDropDownItemActive(item);
        scheduleId = null;
        demoDataId = item;
        initialized = false;
        homeLocationGroup.clearLayers();
        homeLocationMarkerByIdMap.clear();
        visitGroup.clearLayers();
        visitMarkerByIdMap.clear();
        refreshRoutePlan();
      });
    });

    demoDataId = data[0];
    switchDataDropDownItemActive(demoDataId);

    refreshRoutePlan();
  }).fail(function (xhr, ajaxOptions, thrownError) {
    // disable this page as there is no data
    $("#demo").empty();
    $("#demo").html(
      '<h1><p style="justify-content: center">No test data available</p></h1>',
    );
  });
}

function switchDataDropDownItemActive(newItem) {
  activeCssClass = "active";
  $("#testDataButton > a." + activeCssClass).removeClass(activeCssClass);
  $("#" + newItem + "TestData").addClass(activeCssClass);
}

function copyTextToClipboard(id) {
  var text = $("#" + id)
    .text()
    .trim();

  var dummy = document.createElement("textarea");
  document.body.appendChild(dummy);
  dummy.value = text;
  dummy.select();
  document.execCommand("copy");
  document.body.removeChild(dummy);
}

function replaceQuickstartSolverForgeAutoHeaderFooter() {
  const solverforgeHeader = $("header#solverforge-auto-header");
  if (solverforgeHeader != null) {
    solverforgeHeader.css("background-color", "#ffffff");
    solverforgeHeader.append(
      $(`<div class="container-fluid">
        <nav class="navbar sticky-top navbar-expand-lg shadow-sm mb-3" style="background-color: #ffffff;">
          <a class="navbar-brand" href="https://www.solverforge.org">
            <img src="/webjars/solverforge/img/solverforge-horizontal.svg" alt="SolverForge logo" width="400">
          </a>
          <button class="navbar-toggler" type="button" data-toggle="collapse" data-target="#navbarNav" aria-controls="navbarNav" aria-expanded="false" aria-label="Toggle navigation">
            <span class="navbar-toggler-icon"></span>
          </button>
          <div class="collapse navbar-collapse" id="navbarNav">
            <ul class="nav nav-pills">
              <li class="nav-item active" id="navUIItem">
                <button class="nav-link active" id="navUI" data-bs-toggle="pill" data-bs-target="#demo" type="button" style="color: #1f2937;">Demo UI</button>
              </li>
              <li class="nav-item" id="navRestItem">
                <button class="nav-link" id="navRest" data-bs-toggle="pill" data-bs-target="#rest" type="button" style="color: #1f2937;">Guide</button>
              </li>
              <li class="nav-item" id="navOpenApiItem">
                <button class="nav-link" id="navOpenApi" data-bs-toggle="pill" data-bs-target="#openapi" type="button" style="color: #1f2937;">REST API</button>
              </li>
            </ul>
          </div>
          <div class="ms-auto d-flex align-items-center gap-3">
              <div class="form-check form-switch d-flex align-items-center" data-bs-toggle="tooltip" data-bs-placement="bottom" title="Enable real road routing using OpenStreetMap data. Slower initial load (~5-15s for download), but shows accurate road routes instead of straight lines.">
                  <input class="form-check-input" type="checkbox" id="realRoadRouting" checked style="width: 2.5em; height: 1.25em; cursor: pointer;">
                  <label class="form-check-label ms-2" for="realRoadRouting" style="white-space: nowrap; cursor: pointer;">
                      <i class="fas fa-road"></i> Real Roads
                  </label>
              </div>
              <div class="dropdown">
                  <button class="btn dropdown-toggle" type="button" id="dropdownMenuButton" data-bs-toggle="dropdown" aria-haspopup="true" aria-expanded="false" style="background-color: #10b981; color: #ffffff; border-color: #10b981;">
                      Data
                  </button>
                  <div id="testDataButton" class="dropdown-menu" aria-labelledby="dropdownMenuButton"></div>
              </div>
          </div>
        </nav>
      </div>`),
    );
  }

  const solverforgeFooter = $("footer#solverforge-auto-footer");
  if (solverforgeFooter != null) {
    solverforgeFooter.append(
      $(`<footer class="bg-black text-white-50">
               <div class="container">
                 <div class="hstack gap-3 p-4">
                   <div class="ms-auto"><a class="text-white" href="https://www.solverforge.org">SolverForge</a></div>
                   <div class="vr"></div>
                   <div><a class="text-white" href="https://www.solverforge.org/docs">Documentation</a></div>
                   <div class="vr"></div>
                   <div><a class="text-white" href="https://github.com/SolverForge/solverforge-legacy">Code</a></div>
                   <div class="vr"></div>
                   <div class="me-auto"><a class="text-white" href="mailto:info@solverforge.org">Support</a></div>
                 </div>
               </div>
             </footer>`),
    );
  }
}
