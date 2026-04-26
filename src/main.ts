import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";

type OperationMode = "encrypt" | "decrypt";
type PickerMode = "file" | "folder";

interface ProcessRequest {
  path: string;
  password: string;
  mode: OperationMode;
}

interface ProcessResponse {
  output_path: string;
  output_kind: "file" | "folder";
  message: string;
}

const form = document.querySelector<HTMLFormElement>("#encryptallinator-form");
const passwordInput = document.querySelector<HTMLInputElement>("#password-input");
const selectButton = document.querySelector<HTMLButtonElement>("#select-button");
const submitButton = document.querySelector<HTMLButtonElement>("#submit-button");
const selectedPathLabel = document.querySelector<HTMLElement>("#selected-path");
const statusMessage = document.querySelector<HTMLElement>("#status-message");
const pickerMenu = document.querySelector<HTMLElement>("#picker-menu");
const pickerButtons = document.querySelectorAll<HTMLButtonElement>("[data-picker]");

let selectedPath = "";
let currentMode: OperationMode = "encrypt";

function setStatus(message: string, variant: "idle" | "success" | "error" = "idle") {
  if (!statusMessage) {
    return;
  }

  statusMessage.textContent = message;
  statusMessage.dataset.variant = variant;
}

function updateSelectedPath(path: string) {
  selectedPath = path;

  if (selectedPathLabel) {
    selectedPathLabel.textContent = path || "No file or folder selected yet.";
    selectedPathLabel.dataset.empty = path ? "false" : "true";
  }
}

function setBusy(isBusy: boolean) {
  if (submitButton) {
    submitButton.disabled = isBusy;
    submitButton.textContent = isBusy
      ? currentMode === "encrypt"
        ? "Encrypting..."
        : "Decrypting..."
      : currentMode === "encrypt"
        ? "Encrypt"
        : "Decrypt";
  }

  if (selectButton) {
    selectButton.disabled = isBusy;
  }
}

function closePickerMenu() {
  if (!pickerMenu) {
    return;
  }

  pickerMenu.classList.add("hidden");
  pickerMenu.setAttribute("aria-hidden", "true");
}

function togglePickerMenu() {
  if (!pickerMenu) {
    return;
  }

  const nextHiddenState = !pickerMenu.classList.contains("hidden");
  pickerMenu.classList.toggle("hidden", nextHiddenState);
  pickerMenu.setAttribute("aria-hidden", `${nextHiddenState}`);
}

function getCurrentMode(): OperationMode {
  const checkedInput = document.querySelector<HTMLInputElement>('input[name="mode"]:checked');
  return checkedInput?.value === "decrypt" ? "decrypt" : "encrypt";
}

async function choosePath(pickerMode: PickerMode) {
  const selection = await open({
    multiple: false,
    directory: pickerMode === "folder",
    filters:
      pickerMode === "file" && currentMode === "decrypt"
        ? [{ name: "Encryptallinator files", extensions: ["encryptallinator"] }]
        : undefined,
  });

  if (typeof selection === "string") {
    updateSelectedPath(selection);
    setStatus(`Ready to ${currentMode} ${selection}.`);
  }

  closePickerMenu();
}

async function handleSelection() {
  currentMode = getCurrentMode();

  if (currentMode === "decrypt") {
    await choosePath("file");
    return;
  }

  togglePickerMenu();
}

async function handleSubmit(event: SubmitEvent) {
  event.preventDefault();

  if (!passwordInput?.value) {
    setStatus("Enter a password before continuing.", "error");
    return;
  }

  if (!selectedPath) {
    setStatus("Select a file or folder before continuing.", "error");
    return;
  }

  const request: ProcessRequest = {
    path: selectedPath,
    password: passwordInput.value,
    mode: currentMode,
  };

  try {
    setBusy(true);
    setStatus(
      currentMode === "encrypt"
        ? "Encrypting the selected item..."
        : "Decrypting the selected item...",
    );

    const response = await invoke<ProcessResponse>("process_item", { request });
    updateSelectedPath(response.output_path);
    setStatus(response.message, "success");
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    setStatus(message, "error");
  } finally {
    setBusy(false);
  }
}

window.addEventListener("DOMContentLoaded", () => {
  updateSelectedPath("");
  closePickerMenu();
  setBusy(false);

  document.querySelectorAll<HTMLInputElement>('input[name="mode"]').forEach((input) => {
    input.addEventListener("change", () => {
      currentMode = getCurrentMode();
      closePickerMenu();
      updateSelectedPath("");
      setStatus(
        currentMode === "encrypt"
          ? "Choose a file or folder to encrypt."
          : "Choose an Encryptallinator file to decrypt.",
      );
      setBusy(false);
    });
  });

  selectButton?.addEventListener("click", () => {
    void handleSelection();
  });

  pickerButtons.forEach((button) => {
    button.addEventListener("click", () => {
      const pickerMode = button.dataset.picker === "folder" ? "folder" : "file";
      void choosePath(pickerMode);
    });
  });

  document.addEventListener("click", (event) => {
    if (!pickerMenu || !selectButton) {
      return;
    }

    const target = event.target;
    if (target instanceof Node && !pickerMenu.contains(target) && !selectButton.contains(target)) {
      closePickerMenu();
    }
  });

  form?.addEventListener("submit", (event) => {
    void handleSubmit(event);
  });
});
