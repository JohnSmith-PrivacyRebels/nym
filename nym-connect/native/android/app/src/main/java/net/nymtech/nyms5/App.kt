package net.nymtech.nyms5

import android.app.Application
import android.content.Context
import android.util.Log
import androidx.datastore.core.DataStore
import androidx.datastore.preferences.core.Preferences
import androidx.datastore.preferences.core.booleanPreferencesKey
import androidx.datastore.preferences.preferencesDataStore
import androidx.work.Configuration
import androidx.work.DelegatingWorkerFactory
import io.sentry.android.core.SentryAndroid
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.flow.map
import kotlinx.coroutines.runBlocking

val Context.dataStore: DataStore<Preferences> by preferencesDataStore(name = "settings")
val monitoringKey = booleanPreferencesKey("monitoring")

class App : Application(), Configuration.Provider {
    companion object {
        // NymProxy singleton (unique instance)
        val nymProxy = NymProxy()
    }

    private val tag = "App"

    override fun onCreate() {
        super.onCreate()
        val app = this

        runBlocking {
            val monitoring = applicationContext.dataStore.data.map { preferences ->
                preferences[monitoringKey] ?: false
            }.first()

            if (monitoring) {
                Log.i(tag, "Performance monitoring and error reporting enabled")
                SentryAndroid.init(app) { options ->
                    options.dsn =
                        "https://6872f5818bc147ef9c0fce114fcaac8a@o967446.ingest.sentry.io/4505306218102784"
                    options.enableAllAutoBreadcrumbs(true)

                    // TODO should be adjusted in production env
                    options.tracesSampleRate = 1.0
                    options.profilesSampleRate = 1.0
                }
            }
        }
    }

    override fun getWorkManagerConfiguration(): Configuration {
        val workerFactory = DelegatingWorkerFactory()
        // pass in the NymProxy class instance
        workerFactory.addFactory(CustomWorkerFactory(nymProxy))

        Log.d(tag, "using a custom configuration for WorkManager")
        return Configuration.Builder().setWorkerFactory(workerFactory).build()
    }
}