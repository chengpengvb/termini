package app.termini.store

import android.app.Activity
import com.android.billingclient.api.AcknowledgePurchaseParams
import com.android.billingclient.api.BillingClient
import com.android.billingclient.api.BillingClientStateListener
import com.android.billingclient.api.BillingFlowParams
import com.android.billingclient.api.BillingResult
import com.android.billingclient.api.PendingPurchasesParams
import com.android.billingclient.api.ProductDetails
import com.android.billingclient.api.Purchase
import com.android.billingclient.api.PurchasesUpdatedListener
import com.android.billingclient.api.QueryProductDetailsParams
import com.android.billingclient.api.QueryPurchasesParams
import org.json.JSONArray
import org.json.JSONObject
import java.util.concurrent.CountDownLatch
import java.util.concurrent.TimeUnit

class PlayBillingBridge {
  companion object {
    @JvmStatic
    fun listProducts(activity: Activity, ids: String): String {
      val productIds = ids.split(",")
      if (productIds.isEmpty() || productIds.any { it.isEmpty() }) {
        return "ERROR:product ids missing"
      }
      return runClient(activity, 30, "list", PurchasesUpdatedListener { _, _ -> }) { client, done ->
        val products = productIds.map { id ->
          QueryProductDetailsParams.Product.newBuilder()
            .setProductId(id)
            .setProductType(BillingClient.ProductType.SUBS)
            .build()
        }
        val params = QueryProductDetailsParams.newBuilder().setProductList(products).build()
        client.queryProductDetailsAsync(params) { billingResult, queryResult ->
          if (billingResult.responseCode != BillingClient.BillingResponseCode.OK) {
            done(errorText(billingResult))
            return@queryProductDetailsAsync
          }
          if (queryResult.unfetchedProductList.isNotEmpty()) {
            val missed = queryResult.unfetchedProductList[0]
            done("ERROR:unfetched ${missed.productId} ${missed.statusCode}")
            return@queryProductDetailsAsync
          }
          val details = queryResult.productDetailsList
          if (details.isEmpty()) {
            done("ERROR:products missing")
            return@queryProductDetailsAsync
          }
          try {
            done(details.joinToString("\n") { productLine(it) })
          } catch (error: Exception) {
            done("ERROR:$error")
          }
        }
      }
    }

    @JvmStatic
    fun purchase(activity: Activity, productId: String): String {
      if (productId.isEmpty()) {
        return "ERROR:product id missing"
      }
      var doneRef: ((String) -> Unit)? = null
      val listener = PurchasesUpdatedListener { billingResult, purchases ->
        val done = doneRef
        if (done == null) {
          return@PurchasesUpdatedListener
        }
        finishUpdated(billingResult, purchases, done)
      }
      return runClient(activity, 300, "purchase", listener) { client, done ->
        doneRef = done
        val product = QueryProductDetailsParams.Product.newBuilder()
          .setProductId(productId)
          .setProductType(BillingClient.ProductType.SUBS)
          .build()
        val params = QueryProductDetailsParams.newBuilder().setProductList(listOf(product)).build()
        client.queryProductDetailsAsync(params) { billingResult, queryResult ->
          if (billingResult.responseCode != BillingClient.BillingResponseCode.OK) {
            done(errorText(billingResult))
            return@queryProductDetailsAsync
          }
          if (queryResult.unfetchedProductList.isNotEmpty()) {
            val missed = queryResult.unfetchedProductList[0]
            done("ERROR:unfetched ${missed.productId} ${missed.statusCode}")
            return@queryProductDetailsAsync
          }
          val details = queryResult.productDetailsList
          if (details.size != 1) {
            done("ERROR:product missing")
            return@queryProductDetailsAsync
          }
          val offer = firstOffer(details[0])
          if (offer == null) {
            done("ERROR:offer missing")
            return@queryProductDetailsAsync
          }
          val flow = BillingFlowParams.newBuilder()
            .setProductDetailsParamsList(
              listOf(
                BillingFlowParams.ProductDetailsParams.newBuilder()
                  .setProductDetails(details[0])
                  .setOfferToken(offer.offerToken)
                  .build()
              )
            )
            .build()
          val launched = client.launchBillingFlow(activity, flow)
          if (launched.responseCode != BillingClient.BillingResponseCode.OK) {
            done(errorText(launched))
          }
        }
      }
    }

    @JvmStatic
    fun restore(activity: Activity, _unused: String): String {
      return runClient(activity, 30, "restore", PurchasesUpdatedListener { _, _ -> }) { client, done ->
        val params = QueryPurchasesParams.newBuilder().setProductType(BillingClient.ProductType.SUBS).build()
        client.queryPurchasesAsync(params) { billingResult, purchases ->
          if (billingResult.responseCode != BillingClient.BillingResponseCode.OK) {
            done(errorText(billingResult))
            return@queryPurchasesAsync
          }
          try {
            done(restoreJson(purchases))
          } catch (error: Exception) {
            done("ERROR:$error")
          }
        }
      }
    }

    @JvmStatic
    fun acknowledge(activity: Activity, token: String): String {
      if (token.isEmpty()) {
        return "ERROR:purchaseToken missing"
      }
      return runClient(activity, 30, "acknowledge", PurchasesUpdatedListener { _, _ -> }) { client, done ->
        val params = QueryPurchasesParams.newBuilder().setProductType(BillingClient.ProductType.SUBS).build()
        client.queryPurchasesAsync(params) { billingResult, purchases ->
          if (billingResult.responseCode != BillingClient.BillingResponseCode.OK) {
            done(errorText(billingResult))
            return@queryPurchasesAsync
          }
          val purchase = purchases.firstOrNull { it.purchaseToken == token }
          if (purchase == null) {
            done("ERROR:purchase missing")
            return@queryPurchasesAsync
          }
          if (purchase.purchaseState != Purchase.PurchaseState.PURCHASED) {
            done("ERROR:purchase pending")
            return@queryPurchasesAsync
          }
          if (purchase.isAcknowledged) {
            done("OK")
            return@queryPurchasesAsync
          }
          val ackParams = AcknowledgePurchaseParams.newBuilder().setPurchaseToken(token).build()
          client.acknowledgePurchase(ackParams) { ack ->
            if (ack.responseCode != BillingClient.BillingResponseCode.OK) {
              done(errorText(ack))
              return@acknowledgePurchase
            }
            done("OK")
          }
        }
      }
    }

    private fun runClient(
      activity: Activity,
      timeoutSec: Long,
      timeoutName: String,
      listener: PurchasesUpdatedListener,
      afterReady: (BillingClient, (String) -> Unit) -> Unit,
    ): String {
      val latch = CountDownLatch(1)
      var result = "ERROR:$timeoutName not started"
      val holder = arrayOfNulls<BillingClient>(1)
      val done: (String) -> Unit = { value ->
        result = value
        latch.countDown()
      }
      activity.runOnUiThread {
        val client = BillingClient.newBuilder(activity)
          .setListener(listener)
          .enablePendingPurchases(PendingPurchasesParams.newBuilder().enableOneTimeProducts().build())
          .build()
        holder[0] = client
        client.startConnection(object : BillingClientStateListener {
          override fun onBillingSetupFinished(billingResult: BillingResult) {
            if (billingResult.responseCode != BillingClient.BillingResponseCode.OK) {
              done(errorText(billingResult))
              return
            }
            afterReady(client, done)
          }

          override fun onBillingServiceDisconnected() {
          }
        })
      }
      if (!latch.await(timeoutSec, TimeUnit.SECONDS)) {
        result = "ERROR:$timeoutName timeout"
      }
      holder[0]?.endConnection()
      return result
    }

    private fun finishUpdated(
      billingResult: BillingResult,
      purchases: MutableList<Purchase>?,
      done: (String) -> Unit,
    ) {
      if (billingResult.responseCode == BillingClient.BillingResponseCode.USER_CANCELED) {
        done("ERROR:CANCELLED")
        return
      }
      if (billingResult.responseCode != BillingClient.BillingResponseCode.OK) {
        done(errorText(billingResult))
        return
      }
      if (purchases == null || purchases.isEmpty()) {
        done("ERROR:purchase missing")
        return
      }
      val purchase = purchases[0]
      if (purchase.purchaseState != Purchase.PurchaseState.PURCHASED) {
        done("ERROR:purchase pending")
        return
      }
      val token = purchase.purchaseToken
      if (token.isEmpty()) {
        done("ERROR:purchaseToken missing")
        return
      }
      done(token)
    }

    private fun productLine(details: ProductDetails): String {
      val offer = firstOffer(details)
      if (offer == null) {
        throw IllegalStateException("offer missing: ${details.productId}")
      }
      val phases = offer.pricingPhases.pricingPhaseList
      if (phases.isEmpty()) {
        throw IllegalStateException("price missing: ${details.productId}")
      }
      val id = tabField(details.productId)
      val title = tabField(details.title)
      val description = tabField(details.description)
      val price = tabField(phases[0].formattedPrice)
      return "$id\t$title\t$description\t$price"
    }

    private fun firstOffer(details: ProductDetails): ProductDetails.SubscriptionOfferDetails? {
      val offers = details.subscriptionOfferDetails
      if (offers == null || offers.isEmpty()) {
        return null
      }
      return offers[0]
    }

    private fun restoreJson(purchases: List<Purchase>): String {
      val array = JSONArray()
      for (purchase in purchases) {
        if (purchase.products.isEmpty()) {
          throw IllegalStateException("product missing")
        }
        if (purchase.purchaseToken.isEmpty()) {
          throw IllegalStateException("purchaseToken missing")
        }
        val item = JSONObject()
        item.put("productId", purchase.products[0])
        item.put("purchaseToken", purchase.purchaseToken)
        array.put(item)
      }
      return array.toString()
    }

    private fun tabField(value: String): String {
      if (value.isEmpty()) {
        throw IllegalStateException("product field empty")
      }
      if (value.contains('\t') || value.contains('\n')) {
        throw IllegalStateException("product field contains tab or newline")
      }
      return value
    }

    private fun errorText(result: BillingResult): String {
      val message = result.debugMessage
      if (message.isEmpty()) {
        return "ERROR:billing ${result.responseCode}"
      }
      return "ERROR:$message"
    }
  }
}
