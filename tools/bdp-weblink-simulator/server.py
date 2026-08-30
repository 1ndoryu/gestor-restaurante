"""Simulador local, desechable y sin codigo propietario de BDP WebLink.

No escucha fuera de loopback. Reproduce solo el contrato HTTP/JSON observado en
la documentacion y el cliente de Glory. Nunca debe usarse como evidencia de que
el comportamiento real de BDP sea identico.
"""

from __future__ import annotations

import argparse
import copy
import json
import secrets
import threading
import time
from dataclasses import dataclass, field
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
from typing import Any
from urllib.parse import urlparse

LOOPBACK_HOSTS = {"127.0.0.1", "localhost", "::1"}
WRITE_PATHS = {
    "/API/Customers/Create",
    "/API/Orders/Create",
    "/API/Orders/Cancel",
    "/API/Orders/Payment/Add",
    "/API/Orders/Invoice",
    # [198A-1] Escrituras BDP nuevas.
    "/API/Orders/Tip/Add",
    "/API/Waiters/Call",
    "/API/Loyalty/AddPoints",
    "/API/Articles/CreateAndUpdateProfiles",
    "/API/Articles/ModifyPrices",
    "/API/Articles/ModifyAndUpdateProfiles",
    "/API/Departments/Create",
    "/API/Departments/CreateAndUpdateProfiles",
    "/API/Warehouse/CreateFamily",
    "/API/Warehouse/CreateSubfamily",
    "/API/Warehouse/Regularizations",
    "/API/Warehouse/Transfers",
    "/API/Warehouse/UpdateMassiveStock",
    "/API/Warehouse/UpdateStock",
    "/API/Warehouse/UpdateMassiveInventory",
}
PUBLIC_PATHS = {"/Service/Health", "/Auth/Login"}
SENSITIVE_KEYS = {"password", "token", "authorization", "codigointegrador", "integratorcode"}
PERSONAL_KEYS = {"email", "mobilephone", "landlinenumber", "taxid", "address", "addressstreet"}


def _redact(value: Any) -> Any:
    if isinstance(value, dict):
        result = {}
        for key, child in value.items():
            normalized = key.lower().replace("_", "")
            result[key] = "[REDACTED]" if normalized in SENSITIVE_KEYS | PERSONAL_KEYS else _redact(child)
        return result
    if isinstance(value, list):
        return [_redact(item) for item in value]
    return value


def _identifier(payload: dict[str, Any]) -> dict[str, Any]:
    return payload.get("OrderIdentifier") or {}


@dataclass
class SimulatorState:
    fixture_path: Path
    admin_key: str
    lock: threading.RLock = field(default_factory=threading.RLock)
    fixtures: dict[str, Any] = field(default_factory=dict)
    customers: dict[int, dict[str, Any]] = field(default_factory=dict)
    orders: dict[int, dict[str, Any]] = field(default_factory=dict)
    market_index: dict[tuple[int, str], int] = field(default_factory=dict)
    payment_index: dict[str, tuple[int, dict[str, Any]]] = field(default_factory=dict)
    history: list[dict[str, Any]] = field(default_factory=list)
    faults: dict[str, dict[str, Any]] = field(default_factory=dict)
    tokens: set[str] = field(default_factory=set)
    next_order_id: int = 1000
    next_invoice: int = 1
    # [198A-1] Estado de escrituras nuevas.
    articles: dict[int, dict[str, Any]] = field(default_factory=dict)
    departments: dict[int, dict[str, Any]] = field(default_factory=dict)
    families: dict[int, dict[str, Any]] = field(default_factory=dict)
    subfamilies: dict[int, dict[str, Any]] = field(default_factory=dict)
    points: dict[int, float] = field(default_factory=dict)
    stock: dict[int, float] = field(default_factory=dict)
    waiter_calls: list[dict[str, Any]] = field(default_factory=list)
    next_article_id: int = 90000000

    def __post_init__(self) -> None:
        self.reset()

    def reset(self) -> None:
        with self.lock:
            self.fixtures = json.loads(self.fixture_path.read_text(encoding="utf-8"))
            self.customers = {int(item["Code"]): copy.deepcopy(item) for item in self.fixtures["customers"]}
            self.orders = {}
            self.market_index = {}
            self.payment_index = {}
            self.history = []
            self.faults = {}
            self.tokens = set()
            self.next_order_id = 1000
            self.next_invoice = 1
            self.articles = {
                int(item["Code"]): copy.deepcopy(item) for item in self.fixtures.get("articles", [])
            }
            self.departments = {
                int(item.get("Code", item.get("Id"))): copy.deepcopy(item)
                for item in self.fixtures.get("departments", [])
            }
            self.families = {}
            self.subfamilies = {}
            self.points = {}
            self.stock = {
                int(item["Code"]): float(item.get("CurrentStock", 0) or 0)
                for item in self.fixtures.get("articles", [])
            }
            self.waiter_calls = []
            self.next_article_id = 90000000

    def order_for_identifier(self, identifier: dict[str, Any]) -> dict[str, Any] | None:
        if identifier.get("OrderId") is not None:
            return self.orders.get(int(identifier["OrderId"]))
        market_id = identifier.get("MarketId")
        marketplace_id = identifier.get("MarketplaceOrderId")
        if market_id is not None and marketplace_id:
            order_id = self.market_index.get((int(market_id), str(marketplace_id)))
            return self.orders.get(order_id) if order_id is not None else None
        return None


class SimulatorServer(ThreadingHTTPServer):
    daemon_threads = True

    def __init__(self, address: tuple[str, int], state: SimulatorState):
        if address[0] not in LOOPBACK_HOSTS:
            raise ValueError("El simulador solo puede escuchar en loopback")
        super().__init__(address, SimulatorHandler)
        self.state = state


class SimulatorHandler(BaseHTTPRequestHandler):
    server: SimulatorServer

    def log_message(self, fmt: str, *args: Any) -> None:
        return

    def do_POST(self) -> None:  # noqa: N802 - API de BaseHTTPRequestHandler
        path = urlparse(self.path).path
        try:
            length = int(self.headers.get("Content-Length", "0"))
            raw = self.rfile.read(length)
            payload = json.loads(raw or b"{}")
            if not isinstance(payload, dict):
                raise ValueError("el cuerpo debe ser un objeto JSON")
        except (ValueError, json.JSONDecodeError) as error:
            self._json(400, {"ErrorMessage": f"JSON invalido: {error}"})
            return

        if path.startswith("/__simulator/"):
            self._admin(path, payload)
            return

        state = self.server.state
        with state.lock:
            state.history.append({
                "Sequence": len(state.history) + 1,
                "Path": path,
                "Payload": _redact(payload),
                "Authorization": "[REDACTED]" if self.headers.get("Authorization") else None,
                "Write": path in WRITE_PATHS,
            })
            fault = state.faults.pop(path, None)

        if fault and fault.get("delay_ms"):
            time.sleep(min(int(fault["delay_ms"]), 30000) / 1000)
        if fault and fault.get("http_status"):
            self._json(int(fault["http_status"]), {"ErrorMessage": "fallo HTTP simulado"})
            return
        if fault and fault.get("invalid_json"):
            self._raw(200, b"{json-invalido")
            return
        if fault and fault.get("remote_error"):
            self._json(200, {"ErrorMessage": str(fault["remote_error"])})
            return

        if path not in PUBLIC_PATHS and not self._authenticated():
            self._json(401, {"ErrorMessage": "token invalido o ausente"})
            return

        response = self._dispatch(path, payload)
        if fault and fault.get("apply_then_disconnect"):
            self.close_connection = True
            return
        self._json(200 if response is not None else 404, response or {"ErrorMessage": "ruta no simulada"})

    def do_GET(self) -> None:  # noqa: N802
        path = urlparse(self.path).path
        if not path.startswith("/__simulator/") or not self._is_admin():
            self._json(404, {"ErrorMessage": "ruta no simulada"})
            return
        state = self.server.state
        with state.lock:
            if path == "/__simulator/history":
                self._json(200, {"History": copy.deepcopy(state.history)})
            elif path == "/__simulator/state":
                self._json(200, {"Customers": list(state.customers.values()), "Orders": list(state.orders.values())})
            else:
                self._json(404, {"ErrorMessage": "ruta admin no simulada"})

    def _authenticated(self) -> bool:
        value = self.headers.get("Authorization", "")
        return value.startswith("Bearer ") and value[7:] in self.server.state.tokens

    def _is_admin(self) -> bool:
        return secrets.compare_digest(self.headers.get("X-Simulator-Key", ""), self.server.state.admin_key)

    def _admin(self, path: str, payload: dict[str, Any]) -> None:
        if not self._is_admin():
            self._json(403, {"ErrorMessage": "clave de simulador invalida"})
            return
        state = self.server.state
        if path == "/__simulator/reset":
            state.reset()
            self._json(200, {"ErrorMessage": ""})
            return
        if path == "/__simulator/fault":
            target = str(payload.get("Path", ""))
            if not target.startswith("/") or target.startswith("/__simulator/"):
                self._json(400, {"ErrorMessage": "Path de fallo invalido"})
                return
            allowed = {"http_status", "remote_error", "invalid_json", "delay_ms", "apply_then_disconnect"}
            fault = {key: value for key, value in payload.items() if key in allowed}
            with state.lock:
                state.faults[target] = fault
            self._json(200, {"ErrorMessage": ""})
            return
        self._json(404, {"ErrorMessage": "ruta admin no simulada"})

    def _dispatch(self, path: str, payload: dict[str, Any]) -> dict[str, Any] | None:
        state = self.server.state
        with state.lock:
            if path == "/Service/Health":
                return {"IsAlive": True}
            if path == "/Auth/Login":
                required = ("Login", "Password", "CodigoIntegrador")
                if any(not str(payload.get(key, "")).strip() for key in required):
                    return {"ErrorMessage": "credenciales incompletas", "AuthSession": None}
                token = f"sim-{secrets.token_hex(16)}"
                state.tokens.add(token)
                return {"ErrorMessage": "", "AuthSession": {"Token": token, "ExpiresIn_InSecconds": 3540}}
            if path == "/Service/GetVersion":
                return {"Version": 0, "Subversion": 0, "Revision": "SIMULATOR", "Application": "Glory WebLink Simulator", "ApplicationDescription": "Contrato local; no es BDP", "ErrorMessage": ""}
            if path in {"/API/Articles/Export", "/API/Articles/GetPOSList"}:
                return {"Articles": list(copy.deepcopy(state.articles).values()), "ErrorMessage": ""}
            if path == "/API/Customers/Export":
                return {"Customers": list(copy.deepcopy(state.customers).values()), "ErrorMessage": ""}
            if path == "/API/Departments/Export" or path == "/API/Departments/ExportFromProfile":
                return {"Departments": list(copy.deepcopy(state.departments).values()), "ErrorMessage": ""}
            if path in {"/API/Warehouse/GetStock", "/API/Warehouse/GetListStock"}:
                return {"Stock": copy.deepcopy(state.stock), "ErrorMessage": ""}
            if path in {"/API/POSes/Get", "/API/POS/Get"}:
                return {"POSes": copy.deepcopy(state.fixtures["poses"]), "ErrorMessage": ""}
            if path in {"/API/Employees/Get", "/API/Employee/Get", "/API/POS/Employees/Get"}:
                return {"Employees": copy.deepcopy(state.fixtures["employees"]), "ErrorMessage": ""}
            if path in {"/API/Tenders/GetList", "/API/Tenders/GetPOSList"}:
                return {"Tenders": copy.deepcopy(state.fixtures["tenders"]), "ErrorMessage": ""}
            if path in {"/API/Rooms/GetTables", "/API/Room/GetTables"}:
                return {"Rooms": copy.deepcopy(state.fixtures["rooms"]), "Tables": state.fixtures["rooms"][0]["Tables"], "ErrorMessage": ""}
            if path == "/API/Customers/Create":
                return self._create_customer(payload)
            if path == "/API/Orders/Create":
                return self._create_order(payload)
            if path == "/API/Orders/Get":
                order = state.order_for_identifier(_identifier(payload))
                return {"ErrorMessage": "comanda inexistente"} if order is None else {"Order": copy.deepcopy(order), "ErrorMessage": ""}
            if path == "/API/Orders/Cancel":
                order = state.order_for_identifier(_identifier(payload))
                if order is None:
                    return {"ErrorMessage": "comanda inexistente"}
                if order["Status"] == 3:
                    return {"ErrorMessage": "comanda ya facturada"}
                order["Status"] = 2
                return {"Order": copy.deepcopy(order), "ErrorMessage": ""}
            if path == "/API/Orders/Payment/Add":
                return self._add_payment(payload)
            if path == "/API/Orders/Invoice":
                return self._invoice(payload)
            # [198A-1/F2] Lecturas de soporte.
            if path == "/Service/GetApplicationVersion":
                return {"Version": 1, "SubVersion": 0, "Revision": "SIMULATOR", "Application": payload.get("Application"), "ApplicationDescription": "suscripcion simulada", "ErrorMessage": ""}
            if path in {"/API/ProfilesLists/GetCreateArticleList", "/API/ProfilesLists/GetModifyArticleList", "/API/ProfilesLists/GetCreateDepartmentList"}:
                return {"ProfilesList": [{"Profile": 1}], "ErrorMessage": ""}
            if path == "/API/Loyalty/GetPoints":
                return {"Points": state.points.get(int(payload.get("Customer", 0)), 0.0), "ErrorMessage": ""}
            # [198A-1] Escrituras nuevas.
            if path == "/API/Orders/Tip/Add":
                return self._add_tip(payload)
            if path == "/API/Waiters/Call":
                return self._call_waiter(payload)
            if path == "/API/Loyalty/AddPoints":
                return self._add_points(payload)
            if path == "/API/Articles/CreateAndUpdateProfiles":
                return self._create_article(payload)
            if path == "/API/Articles/ModifyPrices":
                return {"ErrorMessage": ""}
            if path == "/API/Articles/ModifyAndUpdateProfiles":
                return self._modify_article(payload)
            if path in {"/API/Departments/Create", "/API/Departments/CreateAndUpdateProfiles"}:
                return self._create_department(payload)
            if path == "/API/Warehouse/CreateFamily":
                return self._create_family(payload)
            if path == "/API/Warehouse/CreateSubfamily":
                return self._create_subfamily(payload)
            if path == "/API/Warehouse/Regularizations":
                return self._regularize(payload)
            if path == "/API/Warehouse/Transfers":
                return self._transfer(payload)
            if path in {"/API/Warehouse/UpdateMassiveStock", "/API/Warehouse/UpdateMassiveInventory"}:
                return self._massive_stock(payload)
            if path == "/API/Warehouse/UpdateStock":
                return self._update_stock(payload)
        return None

    def _create_customer(self, payload: dict[str, Any]) -> dict[str, Any]:
        state = self.server.state
        try:
            code = int(payload["Code"])
        except (KeyError, TypeError, ValueError):
            return {"ErrorMessage": "Code obligatorio"}
        if code <= 0 or not str(payload.get("FiscalName", "")).strip():
            return {"ErrorMessage": "cliente invalido"}
        existing = state.customers.get(code)
        if existing is not None and not payload.get("Overwrite", False):
            return {"ErrorMessage": "codigo de cliente duplicado"}
        state.customers[code] = copy.deepcopy(payload)
        return {"Customer": copy.deepcopy(payload), "ErrorMessage": ""}

    def _create_order(self, payload: dict[str, Any]) -> dict[str, Any]:
        state = self.server.state
        order = payload.get("Order")
        if not isinstance(order, dict):
            return {"ErrorMessage": "Order obligatorio"}
        market_id = order.get("MarketId")
        marketplace_id = str(order.get("MarketplaceOrderId", "")).strip()
        items = order.get("Items")
        if market_id is None or not marketplace_id or not isinstance(items, list) or not items:
            return {"ErrorMessage": "MarketId, MarketplaceOrderId e Items son obligatorios"}
        if any(float(item.get("Units", 0)) <= 0 or float(item.get("Price", -1)) < 0 for item in items):
            return {"ErrorMessage": "linea de comanda invalida"}
        key = (int(market_id), marketplace_id)
        canonical = json.dumps(payload, sort_keys=True, separators=(",", ":"))
        existing_id = state.market_index.get(key)
        if existing_id is not None:
            existing = state.orders[existing_id]
            if existing["_CanonicalRequest"] != canonical:
                return {"ErrorMessage": "MarketplaceOrderId repetido con payload diferente"}
            return {"OrderId": existing_id, "Order": self._public_order(existing), "Duplicate": True, "ErrorMessage": ""}
        checked = int(payload.get("OrderOperationType", 0)) == 1
        if checked:
            return {"OrderId": 0, "Checked": True, "ErrorMessage": ""}
        order_id = state.next_order_id
        state.next_order_id += 1
        stored = copy.deepcopy(order)
        stored.update({"OrderId": order_id, "Status": 0, "Payments": [], "InvoiceNumber": None, "_CanonicalRequest": canonical})
        state.orders[order_id] = stored
        state.market_index[key] = order_id
        return {"OrderId": order_id, "Order": self._public_order(stored), "ErrorMessage": ""}

    @staticmethod
    def _public_order(order: dict[str, Any]) -> dict[str, Any]:
        return {key: copy.deepcopy(value) for key, value in order.items() if not key.startswith("_")}

    def _add_payment(self, payload: dict[str, Any]) -> dict[str, Any]:
        state = self.server.state
        order = state.order_for_identifier(_identifier(payload))
        payment = payload.get("Payment")
        if order is None:
            return {"ErrorMessage": "comanda inexistente"}
        if order["Status"] in {2, 3} or not isinstance(payment, dict):
            return {"ErrorMessage": "comanda no pagable"}
        payment_id = str(payment.get("PaymentId", "")).strip()
        try:
            amount = float(payment.get("Amount", 0))
            tender_id = int(payment.get("TenderId", 0))
        except (TypeError, ValueError):
            return {"ErrorMessage": "pago invalido"}
        if not payment_id or amount <= 0 or tender_id <= 0:
            return {"ErrorMessage": "PaymentId, TenderId y Amount validos son obligatorios"}
        existing = state.payment_index.get(payment_id)
        if existing:
            if existing != (order["OrderId"], payment):
                return {"ErrorMessage": "PaymentId repetido con payload diferente"}
            return {"Order": self._public_order(order), "Duplicate": True, "ErrorMessage": ""}
        total = float(order.get("Total", sum(float(i.get("Total", float(i["Units"]) * float(i["Price"]))) for i in order["Items"])))
        paid = sum(float(item["Amount"]) for item in order["Payments"])
        if amount > round(total - paid, 2):
            return {"ErrorMessage": "importe superior al saldo pendiente"}
        order["Payments"].append(copy.deepcopy(payment))
        state.payment_index[payment_id] = (order["OrderId"], copy.deepcopy(payment))
        return {"Order": self._public_order(order), "Balance": round(total - paid - amount, 2), "ErrorMessage": ""}

    def _invoice(self, payload: dict[str, Any]) -> dict[str, Any]:
        state = self.server.state
        order = state.order_for_identifier(_identifier(payload))
        if order is None:
            return {"ErrorMessage": "comanda inexistente"}
        if order["Status"] == 3:
            return {"InvoiceNumber": order["InvoiceNumber"], "Duplicate": True, "ErrorMessage": ""}
        if order["Status"] == 2:
            return {"ErrorMessage": "comanda cancelada"}
        total = float(order.get("Total", sum(float(i.get("Total", float(i["Units"]) * float(i["Price"]))) for i in order["Items"])))
        paid = sum(float(item["Amount"]) for item in order["Payments"])
        if round(total - paid, 2) != 0:
            return {"ErrorMessage": "comanda con saldo pendiente"}
        invoice = f"SIM-{state.next_invoice:06d}"
        state.next_invoice += 1
        order["Status"] = 3
        order["InvoiceNumber"] = invoice
        return {"InvoiceNumber": invoice, "Order": self._public_order(order), "ErrorMessage": ""}

    # [198A-1] Helpers de las escrituras nuevas.
    @staticmethod
    def _first_article_data(payload: dict[str, Any]) -> dict[str, Any]:
        data = payload.get("ArticleData")
        if isinstance(data, list) and data and isinstance(data[0], dict):
            return data[0]
        if isinstance(data, dict):
            return data
        return {}

    def _create_article(self, payload: dict[str, Any]) -> dict[str, Any]:
        state = self.server.state
        first = self._first_article_data(payload)
        if not first:
            return {"ErrorMessage": "ArticleData obligatorio", "ListaErroresArticulo": []}
        if bool(payload.get("AutomaticCode", False)):
            code = state.next_article_id
            state.next_article_id += 1
        else:
            try:
                code = int(first.get("ArtCode", 0))
            except (TypeError, ValueError):
                return {"ErrorMessage": "ArtCode invalido", "ListaErroresArticulo": []}
        if code <= 0:
            return {"ErrorMessage": "ArtCode invalido", "ListaErroresArticulo": []}
        stored = copy.deepcopy(first)
        stored["Code"] = str(code)
        state.articles[code] = stored
        state.stock.setdefault(code, 0.0)
        return {"ErrorMessage": "", "ListaErroresArticulo": []}

    def _modify_article(self, payload: dict[str, Any]) -> dict[str, Any]:
        state = self.server.state
        first = self._first_article_data(payload)
        try:
            code = int(first.get("ArtCode", 0))
        except (TypeError, ValueError):
            return {"ErrorMessage": "ArtCode invalido"}
        if code in state.articles:
            state.articles[code].update(first)
        return {"ErrorMessage": ""}

    def _create_department(self, payload: dict[str, Any]) -> dict[str, Any]:
        state = self.server.state
        try:
            code = int(payload.get("Code", 0))
        except (TypeError, ValueError):
            return {"ErrorMessage": "Code obligatorio"}
        if code <= 0 or not str(payload.get("Description", "")).strip():
            return {"ErrorMessage": "Code y Description obligatorios"}
        if code in state.departments and not payload.get("Overwrite", False):
            return {"ErrorMessage": "departamento duplicado"}
        state.departments[code] = copy.deepcopy(payload)
        return {"ErrorMessage": ""}

    def _create_family(self, payload: dict[str, Any]) -> dict[str, Any]:
        state = self.server.state
        try:
            code = int(payload.get("Code", 0))
        except (TypeError, ValueError):
            return {"ErrorMessage": "Code obligatorio"}
        if code <= 0 or not str(payload.get("Description", "")).strip():
            return {"ErrorMessage": "Code y Description obligatorios"}
        if code in state.families and not payload.get("Overwrite", False):
            return {"ErrorMessage": "familia duplicada"}
        state.families[code] = copy.deepcopy(payload)
        return {"ErrorMessage": ""}

    def _create_subfamily(self, payload: dict[str, Any]) -> dict[str, Any]:
        state = self.server.state
        try:
            code = int(payload.get("Code", 0))
        except (TypeError, ValueError):
            return {"ErrorMessage": "Code obligatorio"}
        if code <= 0 or not str(payload.get("Description", "")).strip():
            return {"ErrorMessage": "Code y Description obligatorios"}
        if code in state.subfamilies and not payload.get("Overwrite", False):
            return {"ErrorMessage": "subfamilia duplicada"}
        state.subfamilies[code] = copy.deepcopy(payload)
        return {"ErrorMessage": ""}

    def _add_tip(self, payload: dict[str, Any]) -> dict[str, Any]:
        state = self.server.state
        order = state.order_for_identifier(_identifier(payload))
        if order is None:
            return {"ErrorMessage": "comanda inexistente"}
        try:
            amount = float(payload.get("Amount", 0))
        except (TypeError, ValueError):
            return {"ErrorMessage": "Amount invalido"}
        if bool(payload.get("AddTip", False)):
            order["Tip"] = float(order.get("Tip", 0)) + amount
        else:
            order["Tip"] = amount
        return {"Order": self._public_order(order), "ErrorMessage": ""}

    def _call_waiter(self, payload: dict[str, Any]) -> dict[str, Any]:
        state = self.server.state
        state.waiter_calls.append(copy.deepcopy(payload))
        return {"ErrorMessage": ""}

    def _add_points(self, payload: dict[str, Any]) -> dict[str, Any]:
        state = self.server.state
        try:
            customer = int(payload.get("Customer", 0))
            added = float(payload.get("PointsAdded", 0))
        except (TypeError, ValueError):
            return {"ErrorMessage": "Customer y PointsAdded invalidos"}
        if customer <= 0 or not str(payload.get("Reason", "")).strip():
            return {"ErrorMessage": "Customer y Reason obligatorios"}
        state.points[customer] = state.points.get(customer, 0.0) + added
        return {"Points": state.points[customer], "ErrorMessage": ""}

    def _apply_stock_delta(self, article: Any, units: Any) -> None:
        state = self.server.state
        if article is None:
            return
        try:
            code = int(article)
            delta = float(units or 0)
        except (TypeError, ValueError):
            return
        state.stock[code] = state.stock.get(code, 0.0) + delta

    def _regularize(self, payload: dict[str, Any]) -> dict[str, Any]:
        self._apply_stock_delta(payload.get("Article"), payload.get("Units"))
        return {"ErrorMessage": ""}

    def _transfer(self, payload: dict[str, Any]) -> dict[str, Any]:
        self._apply_stock_delta(payload.get("Article"), payload.get("Units"))
        return {"ErrorMessage": ""}

    def _update_stock(self, payload: dict[str, Any]) -> dict[str, Any]:
        self._apply_stock_delta(payload.get("Article"), payload.get("Units"))
        return {"ErrorMessage": ""}

    def _massive_stock(self, payload: dict[str, Any]) -> dict[str, Any]:
        for item in payload.get("ArticlesList", []):
            self._apply_stock_delta(item.get("Article"), item.get("Units"))
        return {"ErrorMessage": "", "ErrorList": []}

    def _json(self, status: int, value: Any) -> None:
        self._raw(status, json.dumps(value, ensure_ascii=False).encode("utf-8"))

    def _raw(self, status: int, body: bytes) -> None:
        self.send_response(status)
        self.send_header("Content-Type", "application/json; charset=utf-8")
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        try:
            self.wfile.write(body)
        except (BrokenPipeError, ConnectionResetError):
            pass


def build_server(host: str, port: int, fixture_path: Path, admin_key: str) -> SimulatorServer:
    if host not in LOOPBACK_HOSTS:
        raise ValueError("Host rechazado: use 127.0.0.1, localhost o ::1")
    if len(admin_key) < 16:
        raise ValueError("La clave admin debe tener al menos 16 caracteres")
    return SimulatorServer((host, port), SimulatorState(fixture_path, admin_key))


def main() -> None:
    parser = argparse.ArgumentParser(description="Simulador local del contrato BDP WebLink")
    parser.add_argument("--host", default="127.0.0.1", choices=sorted(LOOPBACK_HOSTS))
    parser.add_argument("--port", type=int, default=18765)
    parser.add_argument("--fixtures", type=Path, default=Path(__file__).with_name("fixtures.json"))
    parser.add_argument("--admin-key", required=True, help="Clave local de 16+ caracteres")
    args = parser.parse_args()
    server = build_server(args.host, args.port, args.fixtures, args.admin_key)
    print(f"SIMULADOR (NO BDP) escuchando en http://{args.host}:{server.server_port}")
    server.serve_forever()


if __name__ == "__main__":
    main()
