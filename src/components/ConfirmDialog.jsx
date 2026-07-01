import {AlertTriangle} from "lucide-react";
import {formatBytes} from "../lib/format.js";
import Modal from "./Modal.jsx";

const ConfirmDialog = ({image, drive, onConfirm, onCancel}) => {
    return (
        <Modal variant="dialog" className="dialog--danger" onClose={onCancel}>
            <div className="dialog__icon">
                <AlertTriangle size={42} strokeWidth={2.2}/>
            </div>
            <div className="dialog__title">Erase &amp; write?</div>
            <div className="dialog__text">
                This will <strong>permanently erase</strong> everything on the
                selected device and write <strong>{image?.name}</strong>
                {image?.detail ? ` ${image.detail}` : ""} to it.
            </div>
            <div className="dialog__device">
                {drive.name}
                <br/>
                {formatBytes(drive.size)} · {drive.path}
            </div>
            <div className="dialog__actions">
                <button className="pbtn pbtn--ghost" onClick={onCancel}>
                    Cancel
                </button>
                <button className="pbtn" onClick={onConfirm}>
                    Erase &amp; Write
                </button>
            </div>
        </Modal>
    );
}

export default ConfirmDialog;
