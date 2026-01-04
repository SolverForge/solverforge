/**
 * Recommended Fit functionality for adding new visits with recommendations.
 *
 * This module provides:
 * - Modal form for adding new visits
 * - Integration with the recommendation API
 * - Application of selected recommendations
 */

// Customer type configurations (must match CUSTOMER_TYPES in app.js and demo_data.py)
const VISIT_CUSTOMER_TYPES = {
    RESIDENTIAL: { label: "Residential", icon: "fa-home", color: "#10b981", windowStart: "17:00", windowEnd: "20:00", minDemand: 1, maxDemand: 2, minService: 5, maxService: 10 },
    BUSINESS: { label: "Business", icon: "fa-building", color: "#3b82f6", windowStart: "09:00", windowEnd: "17:00", minDemand: 3, maxDemand: 6, minService: 15, maxService: 30 },
    RESTAURANT: { label: "Restaurant", icon: "fa-utensils", color: "#f59e0b", windowStart: "06:00", windowEnd: "10:00", minDemand: 5, maxDemand: 10, minService: 20, maxService: 40 },
};

function addNewVisit(id, lat, lng, map, marker) {
    $('#newVisitModal').modal('show');
    const visitModalContent = $("#newVisitModalContent");
    visitModalContent.children().remove();

    let visitForm = "";

    // Customer Type Selection (prominent at the top)
    visitForm += "<div class='form-group mb-3'>" +
        "  <label class='form-label fw-bold'>Customer Type</label>" +
        "  <div class='row g-2' id='customerTypeButtons'>";

    Object.entries(VISIT_CUSTOMER_TYPES).forEach(([type, config]) => {
        const isDefault = type === 'RESIDENTIAL';
        visitForm += `
            <div class='col-4'>
                <button type='button' class='btn w-100 customer-type-btn ${isDefault ? 'active' : ''}'
                        data-type='${type}'
                        style='border: 2px solid ${config.color}; ${isDefault ? `background-color: ${config.color}; color: white;` : `color: ${config.color};`}'>
                    <i class='fas ${config.icon}'></i><br>
                    <span class='fw-bold'>${config.label}</span><br>
                    <small>${config.windowStart}-${config.windowEnd}</small>
                </button>
            </div>`;
    });

    visitForm += "  </div>" +
        "</div>";

    // Name and Location row
    visitForm += "<div class='form-group mb-3'>" +
        "  <div class='row g-2'>" +
        "    <div class='col-4'>" +
        "      <label for='inputName' class='form-label'>Name</label>" +
        `      <input type='text' class='form-control' id='inputName' value='visit${id}' required>` +
        "      <div class='invalid-feedback'>Field is required</div>" +
        "    </div>" +
        "    <div class='col-4'>" +
        "      <label for='inputLatitude' class='form-label'>Latitude</label>" +
        `      <input type='text' disabled class='form-control' id='inputLatitude' value='${lat.toFixed(6)}'>` +
        "    </div>" +
        "    <div class='col-4'>" +
        "      <label for='inputLongitude' class='form-label'>Longitude</label>" +
        `      <input type='text' disabled class='form-control' id='inputLongitude' value='${lng.toFixed(6)}'>` +
        "    </div>" +
        "  </div>" +
        "</div>";

    // Cargo and Duration row
    visitForm += "<div class='form-group mb-3'>" +
        "  <div class='row g-2'>" +
        "    <div class='col-6'>" +
        "      <label for='inputDemand' class='form-label'>Cargo (units) <small class='text-muted' id='demandHint'>(1-2 typical)</small></label>" +
        "      <input type='number' class='form-control' id='inputDemand' value='1' min='1' required>" +
        "      <div class='invalid-feedback'>Field is required</div>" +
        "    </div>" +
        "    <div class='col-6'>" +
        "      <label for='inputDuration' class='form-label'>Service Duration <small class='text-muted' id='durationHint'>(5-10 min typical)</small></label>" +
        "      <input type='number' class='form-control' id='inputDuration' value='7' min='1' required>" +
        "      <div class='invalid-feedback'>Field is required</div>" +
        "    </div>" +
        "  </div>" +
        "</div>";

    // Time window row
    visitForm += "<div class='form-group mb-3'>" +
        "  <div class='row g-2'>" +
        "    <div class='col-6'>" +
        "      <label for='inputMinStartTime' class='form-label'>Time Window Start</label>" +
        "      <input class='form-control' id='inputMinStartTime' required>" +
        "      <div class='invalid-feedback'>Field is required</div>" +
        "    </div>" +
        "    <div class='col-6'>" +
        "      <label for='inputMaxStartTime' class='form-label'>Time Window End</label>" +
        "      <input class='form-control' id='inputMaxStartTime' required>" +
        "      <div class='invalid-feedback'>Field is required</div>" +
        "    </div>" +
        "  </div>" +
        "</div>";

    visitModalContent.append(visitForm);

    // Initialize with Residential defaults
    const defaultType = VISIT_CUSTOMER_TYPES.RESIDENTIAL;
    const tomorrow = JSJoda.LocalDate.now().plusDays(1);

    function parseTimeToDateTime(timeStr) {
        const [hours, minutes] = timeStr.split(':').map(Number);
        return tomorrow.atTime(JSJoda.LocalTime.of(hours, minutes));
    }

    let minStartPicker = flatpickr("#inputMinStartTime", {
        enableTime: true,
        dateFormat: "Y-m-d H:i",
        defaultDate: parseTimeToDateTime(defaultType.windowStart).format(JSJoda.DateTimeFormatter.ofPattern('yyyy-M-d HH:mm'))
    });

    let maxEndPicker = flatpickr("#inputMaxStartTime", {
        enableTime: true,
        dateFormat: "Y-m-d H:i",
        defaultDate: parseTimeToDateTime(defaultType.windowEnd).format(JSJoda.DateTimeFormatter.ofPattern('yyyy-M-d HH:mm'))
    });

    // Customer type button click handler
    $(".customer-type-btn").click(function() {
        const selectedType = $(this).data('type');
        const config = VISIT_CUSTOMER_TYPES[selectedType];

        // Update button styles
        $(".customer-type-btn").each(function() {
            const btnType = $(this).data('type');
            const btnConfig = VISIT_CUSTOMER_TYPES[btnType];
            $(this).removeClass('active');
            $(this).css({
                'background-color': 'transparent',
                'color': btnConfig.color
            });
        });
        $(this).addClass('active');
        $(this).css({
            'background-color': config.color,
            'color': 'white'
        });

        // Update time windows
        minStartPicker.setDate(parseTimeToDateTime(config.windowStart).format(JSJoda.DateTimeFormatter.ofPattern('yyyy-M-d HH:mm')));
        maxEndPicker.setDate(parseTimeToDateTime(config.windowEnd).format(JSJoda.DateTimeFormatter.ofPattern('yyyy-M-d HH:mm')));

        // Update demand hint and value
        $("#demandHint").text(`(${config.minDemand}-${config.maxDemand} typical)`);
        $("#inputDemand").val(config.minDemand);

        // Update service duration hint and value (use midpoint of range)
        const avgService = Math.round((config.minService + config.maxService) / 2);
        $("#durationHint").text(`(${config.minService}-${config.maxService} min typical)`);
        $("#inputDuration").val(avgService);
    });

    const visitModalFooter = $("#newVisitModalFooter");
    visitModalFooter.children().remove();
    visitModalFooter.append("<button id='recommendationButton' type='button' class='btn btn-success'><i class='fas fa-arrow-right'></i> Get Recommendations</button>");
    $("#recommendationButton").click(getRecommendationsModal);
}

function requestRecommendations(visitId, solution, endpointPath) {
    $.post(endpointPath, JSON.stringify({solution, visitId}), function (recommendations) {
        const visitModalContent = $("#newVisitModalContent");
        visitModalContent.children().remove();

        if (!recommendations || recommendations.length === 0) {
            visitModalContent.append("<div class='alert alert-warning'>No recommendations available. The recommendation API may not be fully implemented.</div>");
            const visitModalFooter = $("#newVisitModalFooter");
            visitModalFooter.children().remove();
            visitModalFooter.append("<button type='button' class='btn btn-secondary' data-bs-dismiss='modal'>Close</button>");
            return;
        }

        let visitOptions = "";
        const visit = solution.visits.find(c => c.id === visitId);

        recommendations.forEach((recommendation, index) => {
            const scoreDiffDisplay = recommendation.scoreDiff || "N/A";
            visitOptions += "<div class='form-check'>" +
                `  <input class='form-check-input' type='radio' name='recommendationOptions' id='option${index}' value='option${index}' ${index === 0 ? 'checked=true' : ''}>` +
                `  <label class='form-check-label' for='option${index}'>` +
                `    Add <b>${visit.name}</b> to vehicle <b>${recommendation.proposition.vehicleId}</b> at position <b>${recommendation.proposition.index + 1}</b> (${scoreDiffDisplay})${index === 0 ? ' - <b>Best Solution</b>': ''}` +
                "  </label>" +
                "</div>";
        });

        visitModalContent.append(visitOptions);

        const visitModalFooter = $("#newVisitModalFooter");
        visitModalFooter.children().remove();
        visitModalFooter.append("<button id='applyRecommendationButton' type='button' class='btn btn-success'><i class='fas fa-check'></i> Accept</button>");
        $("#applyRecommendationButton").click(_ => applyRecommendationModal(recommendations));
    }).fail(function (xhr, ajaxOptions, thrownError) {
        showError("Recommendations request failed.", xhr);
        $('#newVisitModal').modal('hide');
    });
}

function applyRecommendation(solution, visitId, vehicleId, index, endpointPath) {
    $.post(endpointPath, JSON.stringify({solution, visitId, vehicleId, index}), function (updatedSolution) {
        updateSolutionWithNewVisit(updatedSolution);
    }).fail(function (xhr, ajaxOptions, thrownError) {
        showError("Apply recommendation request failed.", xhr);
        $('#newVisitModal').modal('hide');
    });
}
