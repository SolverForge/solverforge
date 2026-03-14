// Header and footer injection for the SolverForge quickstart UI chrome

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
          <div class="ms-auto">
              <div class="dropdown">
                  <button class="btn dropdown-toggle" type="button" id="dropdownMenuButton" data-bs-toggle="dropdown" aria-haspopup="true" aria-expanded="false" style="background-color: #10b981; color: #ffffff; border-color: #10b981;">
                      Data
                  </button>
                  <div id="testDataButton" class="dropdown-menu" aria-labelledby="dropdownMenuButton"></div>
              </div>
          </div>
        </nav>
      </div>`));
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
             </footer>`));
    }
}
