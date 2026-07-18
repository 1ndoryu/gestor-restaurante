import json
import threading
import unittest
import urllib.error
import urllib.request
from pathlib import Path

from server import build_server


ADMIN_KEY = "clave-local-pruebas-seguras"


class SimulatorTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.server = build_server("127.0.0.1", 0, Path(__file__).with_name("fixtures.json"), ADMIN_KEY)
        cls.thread = threading.Thread(target=cls.server.serve_forever, daemon=True)
        cls.thread.start()
        cls.base = f"http://127.0.0.1:{cls.server.server_port}"

    @classmethod
    def tearDownClass(cls):
        cls.server.shutdown()
        cls.server.server_close()

    def setUp(self):
        self.post("/__simulator/reset", {}, admin=True)
        response = self.post("/Auth/Login", {"Login": "local", "Password": "secret", "CodigoIntegrador": "SIM"})
        self.token = response["AuthSession"]["Token"]

    def request(self, method, path, payload=None, *, auth=False, admin=False):
        data = None if payload is None else json.dumps(payload).encode()
        request = urllib.request.Request(self.base + path, data=data, method=method, headers={"Content-Type": "application/json"})
        if auth:
            request.add_header("Authorization", f"Bearer {self.token}")
        if admin:
            request.add_header("X-Simulator-Key", ADMIN_KEY)
        with urllib.request.urlopen(request, timeout=2) as response:
            return json.loads(response.read())

    def post(self, path, payload, **kwargs):
        return self.request("POST", path, payload, **kwargs)

    @staticmethod
    def order_payload(operation=0, marketplace_id="GLOCAL00000001", total=10):
        return {"EmployeeId": 1, "ItemsProfileId": 1, "OrderEndType": 0, "OrderOperationType": operation,
                "Order": {"MarketId": 77, "MarketplaceOrderId": marketplace_id, "PosId": 1, "Total": total,
                          "Items": [{"Id": 1000000000001, "Name": "Articulo", "Units": 1, "Price": total, "Total": total}]}}

    def create_order(self, **kwargs):
        return self.post("/API/Orders/Create", self.order_payload(**kwargs), auth=True)

    def test_rejects_non_loopback_bind(self):
        with self.assertRaises(ValueError):
            build_server("0.0.0.0", 0, Path(__file__).with_name("fixtures.json"), ADMIN_KEY)

    def test_auth_is_required(self):
        with self.assertRaises(urllib.error.HTTPError) as ctx:
            self.post("/API/Orders/Get", {})
        self.assertEqual(ctx.exception.code, 401)

    def test_check_order_does_not_persist(self):
        response = self.create_order(operation=1)
        self.assertTrue(response["Checked"])
        state = self.request("GET", "/__simulator/state", admin=True)
        self.assertEqual(state["Orders"], [])

    def test_create_order_is_idempotent_and_detects_conflict(self):
        first = self.create_order()
        second = self.create_order()
        self.assertEqual(first["OrderId"], second["OrderId"])
        self.assertTrue(second["Duplicate"])
        conflict = self.create_order(total=11)
        self.assertIn("payload diferente", conflict["ErrorMessage"])

    def test_applied_then_disconnected_can_be_reconciled(self):
        self.post("/__simulator/fault", {"Path": "/API/Orders/Create", "apply_then_disconnect": True}, admin=True)
        with self.assertRaises(Exception):
            self.create_order(marketplace_id="GAMBIGUOUS001")
        response = self.post("/API/Orders/Get", {"OrderIdentifier": {"MarketId": 77, "MarketplaceOrderId": "GAMBIGUOUS001"}}, auth=True)
        self.assertEqual(response["Order"]["MarketplaceOrderId"], "GAMBIGUOUS001")

    def test_payment_is_idempotent_and_invoice_requires_full_payment(self):
        order_id = self.create_order()["OrderId"]
        identifier = {"OrderId": order_id}
        early = self.post("/API/Orders/Invoice", {"PosId": 1, "EmployeeId": 1, "OrderIdentifier": identifier}, auth=True)
        self.assertIn("saldo pendiente", early["ErrorMessage"])
        payment = {"OrderIdentifier": identifier, "Payment": {"TenderId": 1, "Amount": 10, "PaymentId": "PAY-LOCAL-1"}}
        self.assertEqual(self.post("/API/Orders/Payment/Add", payment, auth=True)["Balance"], 0)
        self.assertTrue(self.post("/API/Orders/Payment/Add", payment, auth=True)["Duplicate"])
        invoice = self.post("/API/Orders/Invoice", {"PosId": 1, "EmployeeId": 1, "OrderIdentifier": identifier}, auth=True)
        self.assertRegex(invoice["InvoiceNumber"], r"^SIM-\d{6}$")

    def test_history_redacts_credentials_and_personal_data(self):
        self.post("/API/Customers/Create", {"Code": 101, "FiscalName": "Falso", "CommercialName": "Fixture", "MobilePhone": "123", "EMail": "x@example.invalid", "Overwrite": False}, auth=True)
        history = self.request("GET", "/__simulator/history", admin=True)["History"]
        serialized = json.dumps(history)
        self.assertNotIn("x@example.invalid", serialized)
        self.assertNotIn("123", serialized)
        self.assertNotIn(self.token, serialized)
        self.assertIn("[REDACTED]", serialized)

    def test_http_remote_and_invalid_json_faults_are_one_shot(self):
        self.post("/__simulator/fault", {"Path": "/API/Orders/Get", "http_status": 503}, admin=True)
        with self.assertRaises(urllib.error.HTTPError) as ctx:
            self.post("/API/Orders/Get", {"OrderIdentifier": {"OrderId": 1}}, auth=True)
        self.assertEqual(ctx.exception.code, 503)
        self.post("/__simulator/fault", {"Path": "/API/Orders/Get", "remote_error": "fallo funcional"}, admin=True)
        self.assertEqual(self.post("/API/Orders/Get", {}, auth=True)["ErrorMessage"], "fallo funcional")
        self.post("/__simulator/fault", {"Path": "/API/Orders/Get", "invalid_json": True}, admin=True)
        with self.assertRaises(json.JSONDecodeError):
            self.post("/API/Orders/Get", {}, auth=True)


if __name__ == "__main__":
    unittest.main()
