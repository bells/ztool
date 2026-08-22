import React from "react";
import ReactDOM from "react-dom/client";

import { FileEngineApp } from "./FileEngineApp";

const root = document.getElementById("root");
if (!(root instanceof HTMLElement)) {
  throw new Error("The Zero File engine root is missing.");
}

ReactDOM.createRoot(root).render(
  <React.StrictMode>
    <FileEngineApp />
  </React.StrictMode>,
);
