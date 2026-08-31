import React from "react";
import ReactDOM from "react-dom/client";
// Шрифт лежит в проекте, а не тянется из сети: политика содержимого внешние
// шрифты запрещает, да и настольное приложение не должно ждать интернет,
// чтобы нарисовать буквы. Вариативное начертание — один файл на все веса.
import "@fontsource-variable/golos-text";
import App from "./App";
import "./styles.css";

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
