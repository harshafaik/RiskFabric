# DEPRECATED: Replaced by Grafana dashboard (docker/grafana/dashboards/fraud-monitoring.json).
# This file is kept for reference only. See docker-compose.yml for the grafana service.
import streamlit as st
import pandas as pd
import clickhouse_connect
import plotly.express as px
import plotly.graph_objects as go
import os
import time

# --- Setup Page Config ---
st.set_page_config(
    page_title="RiskFabric Fraud Intel Dashboard",
    page_icon="🛡️",
    layout="wide",
    initial_sidebar_state="expanded"
)

# --- Configuration ---
CLICKHOUSE_HOST = os.getenv("CLICKHOUSE_HOST", "localhost")
CLICKHOUSE_USER = "riskfabric_user"
CLICKHOUSE_PASS = "123"
CLICKHOUSE_DB = "riskfabric"

@st.cache_resource
def get_clickhouse_client():
    return clickhouse_connect.get_client(
        host=CLICKHOUSE_HOST,
        username=CLICKHOUSE_USER,
        password=CLICKHOUSE_PASS,
        database=CLICKHOUSE_DB
    )

# --- Header ---
st.title("🛡️ RiskFabric Fraud Intelligence Platform")
st.subheader("Real-Time Ingestion & Fraud Model Scoring Performance")

# --- Sidebar Controls ---
st.sidebar.header("Control Panel")
refresh_rate = st.sidebar.slider("Refresh Rate (seconds)", min_value=1, max_value=10, value=2)
st.sidebar.markdown("---")

# Dynamic Statistics Window (look back window in minutes)
lookback_mins = st.sidebar.slider("Lookback Window (minutes)", min_value=1, max_value=60, value=15)

# --- Query ClickHouse Data ---
try:
    ch_client = get_clickhouse_client()
    
    # 1. Fetch Key Metrics (last X minutes)
    metrics_query = f"""
    SELECT 
        count() as total_count,
        sum(flagged) as total_flagged,
        avg(amount) as avg_amount,
        avg(scored_at - kafka_received_at) * 1000 as avg_latency
    FROM fraud_scores
    WHERE scored_at >= now() - INTERVAL {lookback_mins} MINUTE
    """
    
    metrics_df = ch_client.query_df(metrics_query)
    
    # Extract metrics safety checks
    if not metrics_df.empty and metrics_df['total_count'].iloc[0] > 0:
        total_tx = int(metrics_df['total_count'].iloc[0])
        total_flagged = int(metrics_df['total_flagged'].iloc[0])
        avg_amount = float(metrics_df['avg_amount'].iloc[0])
        avg_latency = float(metrics_df['avg_latency'].iloc[0])
        fraud_rate = (total_flagged / total_tx) * 100
    else:
        total_tx = 0
        total_flagged = 0
        avg_amount = 0.0
        avg_latency = 0.0
        fraud_rate = 0.0

    # 2. Main KPI Layout
    kpi1, kpi2, kpi3, kpi4 = st.columns(4)
    with kpi1:
        st.metric(label="Transactions Scored", value=f"{total_tx:,}", delta=None)
    with kpi2:
        st.metric(
            label="Injected Fraud Detected", 
            value=f"{total_flagged:,}", 
            delta=f"{fraud_rate:.2f}% Rate", 
            delta_color="inverse"
        )
    with kpi3:
        st.metric(label="Average Transaction Value", value=f"${avg_amount:.2f}")
    with kpi4:
        st.metric(label="Avg Pipeline Latency", value=f"{avg_latency:.2f} ms")

    # --- Live Charts ---
    st.markdown("### Real-Time Metrics Visualization")
    
    # 3. Fetch Time Series Data
    ts_query = f"""
    SELECT 
        toStartOfSecond(scored_at) as time_bucket,
        count() as tx_count,
        sum(flagged) as fraud_count,
        avg(scored_at - kafka_received_at) * 1000 as latency
    FROM fraud_scores
    WHERE scored_at >= now() - INTERVAL {lookback_mins} MINUTE
    GROUP BY time_bucket
    ORDER BY time_bucket ASC
    """
    ts_df = ch_client.query_df(ts_query)
    
    col1, col2 = st.columns(2)
    
    with col1:
        st.markdown("#### Throughput & Fraud Detection")
        if not ts_df.empty:
            fig_tp = go.Figure()
            fig_tp.add_trace(go.Scatter(
                x=ts_df['time_bucket'], y=ts_df['tx_count'],
                mode='lines', name='Total Transactions', line=dict(color='#00CC96', width=2)
            ))
            fig_tp.add_trace(go.Bar(
                x=ts_df['time_bucket'], y=ts_df['fraud_count'],
                name='Flagged Fraud', marker_color='#EF553B'
            ))
            fig_tp.update_layout(
                margin=dict(l=20, r=20, t=20, b=20),
                legend=dict(orientation="h", yanchor="bottom", y=1.02, xanchor="right", x=1),
                height=350,
                paper_bgcolor="rgba(0,0,0,0)",
                plot_bgcolor="rgba(0,0,0,0)"
            )
            st.plotly_chart(fig_tp, use_container_width=True)
        else:
            st.info("Awaiting streaming data...")

    with col2:
        st.markdown("#### End-to-End Latency Profile")
        if not ts_df.empty:
            fig_lat = px.line(
                ts_df, x='time_bucket', y='latency',
                labels={'latency': 'Latency (ms)', 'time_bucket': 'Time'},
                color_discrete_sequence=['#636EFA']
            )
            fig_lat.update_layout(
                margin=dict(l=20, r=20, t=20, b=20),
                height=350,
                paper_bgcolor="rgba(0,0,0,0)",
                plot_bgcolor="rgba(0,0,0,0)"
            )
            st.plotly_chart(fig_lat, use_container_width=True)
        else:
            st.info("Awaiting streaming data...")

    # --- Flagged Queue ---
    st.markdown("### 🚨 Recent Flagged Fraud Queue")
    flagged_query = """
    SELECT 
        transaction_id,
        card_id,
        customer_id,
        amount,
        timestamp as tx_time,
        fraud_probability as score,
        scored_at
    FROM fraud_scores
    WHERE flagged = 1
    ORDER BY scored_at DESC
    LIMIT 10
    """
    flagged_df = ch_client.query_df(flagged_query)
    
    if not flagged_df.empty:
        # Format columns for presentation
        flagged_df['score'] = flagged_df['score'].map(lambda x: f"{x * 100:.1f}%")
        flagged_df['amount'] = flagged_df['amount'].map(lambda x: f"${x:.2f}")
        st.dataframe(flagged_df, use_container_width=True)
    else:
        st.success("No recent transactions flagged as fraud (System Normal)")

except Exception as e:
    st.error(f"Failed to connect to ClickHouse: {e}")
    st.info("Ensure the ClickHouse server is running and accessible.")

# --- Auto-refresh ---
time.sleep(refresh_rate)
st.rerun()
