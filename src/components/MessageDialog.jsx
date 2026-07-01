import {CheckCircle2, XCircle} from "lucide-react";
import Modal from "./Modal.jsx";

const MessageDialog = ({kind, title, text, onClose}) => {
    const good = kind === "good";
    const Icon = good ? CheckCircle2 : XCircle;
    return (
        <Modal variant="dialog" className={good ? "dialog--good" : "dialog--danger"} onClose={onClose}>
            <div className="dialog__icon">
                <Icon size={44} strokeWidth={2.2}/>
            </div>
            <div className="dialog__title">{title}</div>
            <div className="dialog__text">{text}</div>
            <div className="dialog__actions" style={{justifyContent: "center"}}>
                <button className="pbtn" style={{flex: 1}} onClick={onClose}>
                    OK
                </button>
            </div>
        </Modal>
    );
}

export default MessageDialog;
