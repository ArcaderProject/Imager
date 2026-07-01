import {getCurrentWindow} from "@tauri-apps/api/window";
import {Minus, X} from "lucide-react";

const appWindow = getCurrentWindow();

const TitleBar = () => {
    return (
        <div className="titlebar" data-tauri-drag-region>
            <div className="titlebar__title" data-tauri-drag-region>
                ARCADER IMAGER
            </div>
            <div className="titlebar__controls">
                <button
                    className="titlebar__btn"
                    onClick={() => appWindow.minimize()}
                    aria-label="Minimize"
                >
                    <Minus size={15}/>
                </button>
                <button
                    className="titlebar__btn titlebar__btn--close"
                    onClick={() => appWindow.close()}
                    aria-label="Close"
                >
                    <X size={16}/>
                </button>
            </div>
        </div>
    );
}

export default TitleBar;
