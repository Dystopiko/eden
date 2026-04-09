macro_rules! metrics {
    {
        $vis:vis struct $name:ident {
            $( $namespace:expr => {$(
                #[doc = $help:expr]
                $( #[$meta:meta] )*
                pub $metric:ident : $ty:ident
                $( [ $( $label:expr ),* ] )?,
                $( = [ $( $expr:expr ),* ] )?
            )*}, )*
        }
    } => {
        #[derive(Debug, Clone)]
        $vis struct $name {
            $( $( #[doc = $help] pub $metric: $ty, )* )*
            registry: ::prometheus::Registry,
        }

        impl $name {
            pub fn new() -> core::result::Result<Self, ::prometheus::Error> {
                let registry = ::prometheus::Registry::new();

                $($(
                    let $metric: $ty = crate::macros::types_to_opts::$ty::new(stringify!($metric), $help)
                        .namespace($namespace)
                        $( .variable_labels(vec![ $( $label.into() ),* ]) )?
                        $( .buckets(vec![ $( $expr ),* ]) )?
                        .into_metric_type()?
                        ;

                    registry.register(Box::new($metric.clone()))?;
                )*)*

                Ok(Self {
                    $($( $metric, )*)*
                    registry,
                })
            }

            pub fn encode(&self) -> core::result::Result<std::string::String, ::error_stack::Report<crate::EncodeError>> {
                use ::error_stack::ResultExt;

                let encoder = ::prometheus::TextEncoder::new();
                let families = self.registry.gather();
                encoder
                    .encode_to_string(&families)
                    .change_context(crate::EncodeError)
            }
        }
    };
}
pub(crate) use metrics;
use prometheus::{Histogram, HistogramOpts, HistogramVec};

#[allow(unused)]
#[doc(hidden)]
pub(crate) mod types_to_opts {
    pub type Counter = prometheus::Opts;
    pub type CounterVec = prometheus::Opts;

    pub type IntCounter = prometheus::Opts;
    pub type IntCounterVec = prometheus::Opts;

    pub type Gauge = prometheus::Opts;
    pub type GaugeVec = prometheus::Opts;

    pub type IntGauge = prometheus::Opts;
    pub type IntGaugeVec = prometheus::Opts;

    pub type Histogram = prometheus::HistogramOpts;
    pub type HistogramVec = prometheus::HistogramOpts;
}

pub(super) trait IntoMetricType<T>: Sized {
    fn into_metric_type(self) -> Result<T, prometheus::Error>;
}

macro_rules! impl_primitive {
    ($name:ident as single) => {
        use prometheus::$name;
        impl IntoMetricType<$name> for prometheus::Opts {
            fn into_metric_type(self) -> Result<$name, prometheus::Error> {
                $name::with_opts(self.into())
            }
        }
    };
    ($name:ident as vec) => {
        use prometheus::$name;
        impl IntoMetricType<$name> for prometheus::Opts {
            fn into_metric_type(self) -> Result<$name, prometheus::Error> {
                $name::new(
                    self.clone().into(),
                    self.variable_labels
                        .iter()
                        .map(|s| s.as_str())
                        .collect::<Vec<_>>()
                        .as_slice(),
                )
            }
        }
    };
}
// end of copy

impl IntoMetricType<Histogram> for prometheus::HistogramOpts {
    fn into_metric_type(self) -> Result<Histogram, prometheus::Error> {
        Histogram::with_opts(self)
    }
}

impl IntoMetricType<HistogramVec> for prometheus::HistogramOpts {
    fn into_metric_type(self) -> Result<HistogramVec, prometheus::Error> {
        HistogramVec::new(
            HistogramOpts {
                common_opts: self.common_opts.clone(),
                buckets: self.buckets,
            },
            self.common_opts
                .variable_labels
                .iter()
                .map(|s| s.as_str())
                .collect::<Vec<_>>()
                .as_slice(),
        )
    }
}

impl_primitive!(Counter as single);
impl_primitive!(CounterVec as vec);
impl_primitive!(IntCounter as single);
impl_primitive!(IntCounterVec as vec);
impl_primitive!(Gauge as single);
impl_primitive!(GaugeVec as vec);
impl_primitive!(IntGauge as single);
impl_primitive!(IntGaugeVec as vec);
