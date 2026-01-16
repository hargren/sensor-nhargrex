package com.nhargrex.sensor

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.viewModels
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import com.nhargrex.sensor.ui.theme.SensorTheme
import com.patrykandpatrick.vico.compose.cartesian.rememberCartesianChart
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.unit.sp
import androidx.lifecycle.ViewModel
import androidx.lifecycle.ViewModelProvider
import androidx.lifecycle.viewModelScope
import com.patrykandpatrick.vico.compose.cartesian.CartesianChartHost
import com.patrykandpatrick.vico.compose.cartesian.axis.rememberAxisLabelComponent
import com.patrykandpatrick.vico.compose.cartesian.axis.rememberBottom
import com.patrykandpatrick.vico.compose.cartesian.axis.rememberStart
import com.patrykandpatrick.vico.compose.cartesian.layer.rememberLineCartesianLayer
import com.patrykandpatrick.vico.compose.common.component.rememberShapeComponent
import com.patrykandpatrick.vico.compose.common.component.rememberTextComponent
import com.patrykandpatrick.vico.compose.common.fill
import com.patrykandpatrick.vico.compose.common.rememberHorizontalLegend
import com.patrykandpatrick.vico.core.cartesian.axis.HorizontalAxis
import com.patrykandpatrick.vico.core.cartesian.axis.VerticalAxis
import com.patrykandpatrick.vico.core.cartesian.data.CartesianChartModelProducer
import com.patrykandpatrick.vico.core.cartesian.data.CartesianValueFormatter
import com.patrykandpatrick.vico.core.cartesian.data.lineSeries
import com.patrykandpatrick.vico.core.common.LegendItem
import com.patrykandpatrick.vico.core.cartesian.CartesianDrawingContext
import com.patrykandpatrick.vico.core.cartesian.CartesianMeasuringContext
import com.patrykandpatrick.vico.core.cartesian.data.CartesianLayerRangeProvider
import com.patrykandpatrick.vico.core.common.Insets
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.launch
import java.text.DecimalFormat

class HistoryChartViewModel(
    private val dao: SensorHistoryDao
) : ViewModel() {

    private val _chartData = MutableStateFlow<List<SensorHistoryEntity>>(emptyList())
    val chartData: StateFlow<List<SensorHistoryEntity>> = _chartData

    init {
        loadChartData()
    }

    private fun loadChartData() {
        viewModelScope.launch {
            _chartData.value = dao.getLatest1024()
        }
    }
}

class HistoryChartViewModelFactory(
    private val dao: SensorHistoryDao
) : ViewModelProvider.Factory {

    override fun <T : ViewModel> create(modelClass: Class<T>): T {
        if (modelClass.isAssignableFrom(HistoryChartViewModel::class.java)) {
            @Suppress("UNCHECKED_CAST")
            return HistoryChartViewModel(dao) as T
        }
        throw IllegalArgumentException("Unknown ViewModel class")
    }
}

class HistoryChartActivity : ComponentActivity() {

    private val viewModel: HistoryChartViewModel by viewModels {
        HistoryChartViewModelFactory(
            (application as SensorApp).database.sensorHistoryDao()
        )
    }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

        setContent {
            SensorTheme {
                val history by viewModel.chartData.collectAsState()

                if (history.isNotEmpty()) {
                    TempHumidityChart(history)
                }
            }
        }
    }
}
@Composable
fun TempHumidityChart(history: List<SensorHistoryEntity>) {
    val modelProducer = remember { CartesianChartModelProducer() }

    val yStep = 10.0

    val startAxisValueFormatter = CartesianValueFormatter.decimal(DecimalFormat("#,##0"))

    val startAxisItemPlacer = VerticalAxis.ItemPlacer.step({ yStep })

    LaunchedEffect(history) {
        modelProducer.runTransaction {
            val tempSeries = history.map { it.temp }
            val humiditySeries = history.map { it.humidity }

            lineSeries {
                series(tempSeries)
                series(humiditySeries)
            }
        }
    }

    // Pre-create components OUTSIDE the lambda (these are @Composable)
    val tempIcon = rememberShapeComponent(
        shape = com.patrykandpatrick.vico.core.common.shape.Shape.Rectangle,
        fill = fill(Color.Blue)
    )

    val humidityIcon = rememberShapeComponent(
        shape = com.patrykandpatrick.vico.core.common.shape.Shape.Rectangle,
        fill = fill(Color.Green)
    )

    val labelComponent = rememberTextComponent(
        textSize = 12.sp
    )

    val legend = rememberHorizontalLegend<CartesianMeasuringContext, CartesianDrawingContext>(
        items = { _ ->
            add(
                LegendItem(
                    icon = tempIcon,
                    labelComponent = labelComponent,
                    label = "Temperature (°F)"
                )
            )
            add(
                LegendItem(
                    icon = humidityIcon,
                    labelComponent = labelComponent,
                    label = "Humidity (%)"
                )
            )
        },
        iconSize = 12.dp,
        iconLabelSpacing = 8.dp,
        padding = Insets(8.0f)
    )

    val fixedRangeProvider = remember {
        CartesianLayerRangeProvider.fixed(
            minY = 20.0,
            maxY = 80.0
        )
    }

    val lineCartesianLayer = rememberLineCartesianLayer(
        rangeProvider = fixedRangeProvider
    )

    Column(
        modifier = Modifier.fillMaxWidth()
    ) {
        Text(
            text = "Temperature and Humidity History",
            style = MaterialTheme.typography.titleLarge,
            modifier = Modifier
                .padding(top = 8.dp, bottom = 8.dp)
                .align(Alignment.CenterHorizontally)
        )

        CartesianChartHost(
            chart = rememberCartesianChart(
                lineCartesianLayer,
                startAxis = VerticalAxis.rememberStart(
                    label = rememberAxisLabelComponent(color = Color.Black),
                    valueFormatter = startAxisValueFormatter,
                    itemPlacer = startAxisItemPlacer,
                ),
                bottomAxis = HorizontalAxis.rememberBottom(),
                legend = legend
            ),
            modelProducer = modelProducer,
            modifier = Modifier
                .fillMaxWidth()
                .height(300.dp)
        )
    }
}





