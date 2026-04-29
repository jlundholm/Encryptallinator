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
const submitButton = document.querySelector<HTMLButtonElement>("#submit-button");
const selectedPathLabel = document.querySelector<HTMLElement>("#selected-path");
const statusMessage = document.querySelector<HTMLElement>("#status-message");
const pickerButtons = document.querySelectorAll<HTMLButtonElement>("[data-picker]");
const folderPickerButton = document.querySelector<HTMLButtonElement>('[data-picker="folder"]');

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

  pickerButtons.forEach((button) => {
    const pickerMode = button.dataset.picker === "folder" ? "folder" : "file";
    button.disabled = isBusy || (currentMode === "decrypt" && pickerMode === "folder");
  });
}

function getCurrentMode(): OperationMode {
  const checkedInput = document.querySelector<HTMLInputElement>('input[name="mode"]:checked');
  return checkedInput?.value === "decrypt" ? "decrypt" : "encrypt";
}

function updatePickerAvailability() {
  if (!folderPickerButton) {
    return;
  }

  const decryptMode = currentMode === "decrypt";
  folderPickerButton.disabled = decryptMode;
  folderPickerButton.setAttribute("aria-disabled", `${decryptMode}`);
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
  currentMode = getCurrentMode();
  updateSelectedPath("");
  updatePickerAvailability();
  setBusy(false);

  document.querySelectorAll<HTMLInputElement>('input[name="mode"]').forEach((input) => {
    input.addEventListener("change", () => {
      currentMode = getCurrentMode();
      updatePickerAvailability();
      updateSelectedPath("");
      setStatus(
        currentMode === "encrypt"
          ? "Choose a file or folder to encrypt."
          : "Choose an Encryptallinator file to decrypt.",
      );
      setBusy(false);
    });
  });

  pickerButtons.forEach((button) => {
    button.addEventListener("click", () => {
      const pickerMode = button.dataset.picker === "folder" ? "folder" : "file";
      void choosePath(pickerMode);
    });
  });

  form?.addEventListener("submit", (event) => {
    void handleSubmit(event);
  });
});
