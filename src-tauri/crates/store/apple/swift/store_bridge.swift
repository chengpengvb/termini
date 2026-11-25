import Foundation
import StoreKit

private struct BridgeProduct: Encodable {
    let productId: String
    let title: String
    let description: String
    let priceText: String
}

private struct BridgeTransaction: Encodable {
    let productId: String
    let originalTransactionId: String
    let jws: String
}

public typealias BridgeCallback = @convention(c) (UnsafeMutableRawPointer, Int32, UnsafePointer<CChar>?) -> Void

private func deliver(_ context: UInt, _ callback: BridgeCallback, _ code: Int32, _ payload: String) {
    let pointer = UnsafeMutableRawPointer(bitPattern: context)!
    payload.withCString { callback(pointer, code, $0) }
}

private func encode<T: Encodable>(_ value: T) throws -> String {
    String(decoding: try JSONEncoder().encode(value), as: UTF8.self)
}

private func bridgeTransaction(_ transaction: Transaction, _ jws: String) -> BridgeTransaction {
    BridgeTransaction(productId: transaction.productID, originalTransactionId: String(transaction.originalID), jws: jws)
}

private func collect(_ sequence: Transaction.Transactions, into items: inout [String: BridgeTransaction]) async {
    for await result in sequence {
        guard case .verified(let transaction) = result else { continue }
        items[String(transaction.originalID)] = bridgeTransaction(transaction, result.jwsRepresentation)
    }
}

@_cdecl("store_bridge_list_products")
public func storeBridgeListProducts(_ idsJson: UnsafePointer<CChar>, _ context: UnsafeMutableRawPointer, _ callback: @escaping BridgeCallback) {
    let raw = String(cString: idsJson)
    let contextBits = UInt(bitPattern: context)
    Task {
        do {
            let ids = try JSONDecoder().decode([String].self, from: Data(raw.utf8))
            let products = try await Product.products(for: ids)
            let items = products.map { BridgeProduct(productId: $0.id, title: $0.displayName, description: $0.description, priceText: $0.displayPrice) }
            deliver(contextBits, callback, 0, try encode(items))
        } catch {
            deliver(contextBits, callback, 2, error.localizedDescription)
        }
    }
}

@_cdecl("store_bridge_purchase")
public func storeBridgePurchase(_ productId: UnsafePointer<CChar>, _ context: UnsafeMutableRawPointer, _ callback: @escaping BridgeCallback) {
    let id = String(cString: productId)
    let contextBits = UInt(bitPattern: context)
    Task {
        do {
            guard let product = try await Product.products(for: [id]).first else {
                deliver(contextBits, callback, 2, "商店中不存在商品: \(id)")
                return
            }
            let result = try await product.purchase()
            switch result {
            case .success(let verification):
                switch verification {
                case .verified(let transaction):
                    deliver(contextBits, callback, 0, try encode(bridgeTransaction(transaction, verification.jwsRepresentation)))
                case .unverified(_, let error):
                    deliver(contextBits, callback, 2, "交易校验失败: \(error.localizedDescription)")
                }
            case .userCancelled:
                deliver(contextBits, callback, 1, "")
            case .pending:
                deliver(contextBits, callback, 2, "购买等待批准，批准后会自动开通")
            @unknown default:
                deliver(contextBits, callback, 2, "未知购买结果")
            }
        } catch {
            deliver(contextBits, callback, 2, error.localizedDescription)
        }
    }
}

@_cdecl("store_bridge_finish")
public func storeBridgeFinish(_ originalId: UnsafePointer<CChar>, _ context: UnsafeMutableRawPointer, _ callback: @escaping BridgeCallback) {
    let id = String(cString: originalId)
    let contextBits = UInt(bitPattern: context)
    Task {
        for await result in Transaction.unfinished {
            guard case .verified(let transaction) = result, String(transaction.originalID) == id else { continue }
            await transaction.finish()
        }
        deliver(contextBits, callback, 0, "")
    }
}

@_cdecl("store_bridge_unfinished")
public func storeBridgeUnfinished(_ context: UnsafeMutableRawPointer, _ callback: @escaping BridgeCallback) {
    let contextBits = UInt(bitPattern: context)
    Task {
        do {
            var items: [String: BridgeTransaction] = [:]
            await collect(Transaction.unfinished, into: &items)
            deliver(contextBits, callback, 0, try encode(Array(items.values)))
        } catch {
            deliver(contextBits, callback, 2, error.localizedDescription)
        }
    }
}

@_cdecl("store_bridge_restore")
public func storeBridgeRestore(_ context: UnsafeMutableRawPointer, _ callback: @escaping BridgeCallback) {
    let contextBits = UInt(bitPattern: context)
    Task {
        do {
            try await AppStore.sync()
            var items: [String: BridgeTransaction] = [:]
            await collect(Transaction.currentEntitlements, into: &items)
            await collect(Transaction.unfinished, into: &items)
            deliver(contextBits, callback, 0, try encode(Array(items.values)))
        } catch {
            deliver(contextBits, callback, 2, error.localizedDescription)
        }
    }
}

@_cdecl("store_bridge_listen")
public func storeBridgeListen(_ context: UnsafeMutableRawPointer, _ callback: @escaping BridgeCallback) {
    let contextBits = UInt(bitPattern: context)
    Task {
        for await result in Transaction.updates {
            guard case .verified(let transaction) = result else { continue }
            do {
                deliver(contextBits, callback, 0, try encode(bridgeTransaction(transaction, result.jwsRepresentation)))
            } catch {
                deliver(contextBits, callback, 2, error.localizedDescription)
            }
        }
    }
}
